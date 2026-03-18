use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use semantit_core::error::ParseError;
use semantit_core::hash::{
    build_entity_id, build_full_hash, build_shape_hash, hash_string, normalize_whitespace,
};
use semantit_core::model::{
    EntityKind, ExportBinding, FileSemanticIndex, LanguageId, SemanticEntity, SpanLocation,
    SpanRange,
};
use semantit_core::parser::{LanguageParser, ParseInput};
use serde_json::Value;
use swc_common::sync::Lrc;
use swc_common::{BytePos, FileName, SourceMap, SourceMapper, Span};
use swc_ecma_ast::{
    ArrowExpr, BindingIdent, BlockStmtOrExpr, Class, ClassMember, ClassMethod, Constructor, Decl,
    DefaultDecl, ExportDecl, ExportDefaultDecl, ExportDefaultExpr, ExportSpecifier, Expr, Function,
    Ident, IdentName, MethodKind, Module, ModuleDecl, ModuleExportName, ModuleItem, Param,
    ParamOrTsParamProp, Pat, PrivateMethod, PrivateName, PropName, TsInterfaceDecl,
    TsTypeAliasDecl, VarDecl, VarDeclKind, VarDeclarator,
};
use swc_ecma_codegen::text_writer::JsWriter;
use swc_ecma_codegen::{Config as CodegenConfig, Emitter, Node};
use swc_ecma_parser::lexer::Lexer;
use swc_ecma_parser::{Parser, StringInput, Syntax, TsSyntax};
use swc_ecma_visit::{Visit, VisitWith};

const SIGNATURE_SHAPE_KEY: &str = "signature_shape";
const PARENT_ANCHOR_KEY: &str = "parent_anchor";
const SOURCE_KIND_KEY: &str = "source_kind";
const MODULE_ANCHOR: &str = "<module>";

#[derive(Debug, Default, Clone, Copy)]
pub struct TypeScriptParser;

impl LanguageParser for TypeScriptParser {
    fn language(&self) -> LanguageId {
        LanguageId::TypeScript
    }

    fn parser_version(&self) -> &'static str {
        concat!(
            env!("CARGO_PKG_NAME"),
            "@",
            env!("CARGO_PKG_VERSION"),
            "+swc_ecma_parser@35.0.0"
        )
    }

    fn supports_path(&self, path: &Path) -> bool {
        matches!(
            path.extension().and_then(|value| value.to_str()),
            Some("ts" | "tsx" | "mts" | "cts")
        )
    }

    fn parse(&self, input: ParseInput<'_>) -> Result<FileSemanticIndex, ParseError> {
        let path_str = input.path.display().to_string();
        let cm: Lrc<SourceMap> = Default::default();
        let file = cm.new_source_file(
            FileName::Real(input.path.to_path_buf()).into(),
            input.source.to_string(),
        );

        let syntax = Syntax::Typescript(TsSyntax {
            tsx: matches!(
                input.path.extension().and_then(|value| value.to_str()),
                Some("tsx")
            ),
            decorators: true,
            ..Default::default()
        });

        let lexer = Lexer::new(syntax, Default::default(), StringInput::from(&*file), None);
        let mut parser = Parser::new_from(lexer);
        let module =
            parser
                .parse_typescript_module()
                .map_err(|error| ParseError::ParserFailure {
                    language: LanguageId::TypeScript,
                    path: path_str.clone(),
                    message: format!("{error:?}"),
                })?;

        let parse_errors = parser
            .take_errors()
            .into_iter()
            .map(|error| format!("{error:?}"))
            .collect::<Vec<_>>();

        if !parse_errors.is_empty() {
            return Err(ParseError::ParserFailure {
                language: LanguageId::TypeScript,
                path: path_str,
                message: parse_errors.join("; "),
            });
        }

        TypeScriptIndexBuilder::new(input.path, input.source, cm)
            .build(&module, self.parser_version())
    }
}

#[derive(Clone)]
struct ParentContext {
    id: String,
    path: String,
}

#[derive(Debug, Clone)]
struct ExportIntent {
    exported_name: String,
    local_name: String,
    entity_id: Option<String>,
    is_default: bool,
    type_only: bool,
    source: Option<String>,
}

struct TypeScriptIndexBuilder<'a> {
    path: &'a Path,
    source: &'a str,
    cm: Lrc<SourceMap>,
    entities: Vec<SemanticEntity>,
    name_to_id: BTreeMap<String, String>,
    export_intents: Vec<ExportIntent>,
}

impl<'a> TypeScriptIndexBuilder<'a> {
    fn new(path: &'a Path, source: &'a str, cm: Lrc<SourceMap>) -> Self {
        Self {
            path,
            source,
            cm,
            entities: Vec::new(),
            name_to_id: BTreeMap::new(),
            export_intents: Vec::new(),
        }
    }

    fn build(
        mut self,
        module: &Module,
        parser_version: &str,
    ) -> Result<FileSemanticIndex, ParseError> {
        for item in &module.body {
            self.collect_module_item(item)?;
        }

        self.entities
            .sort_by(|left, right| left.path.cmp(&right.path));

        let root_entities = self
            .entities
            .iter()
            .filter(|entity| entity.parent.is_none())
            .map(|entity| entity.id.clone())
            .collect::<Vec<_>>();

        let mut exports = self
            .export_intents
            .into_iter()
            .map(|intent| ExportBinding {
                exported_name: intent.exported_name,
                local_name: intent.local_name.clone(),
                entity_id: intent
                    .entity_id
                    .or_else(|| self.name_to_id.get(&intent.local_name).cloned()),
                is_default: intent.is_default,
                type_only: intent.type_only,
                source: intent.source,
            })
            .collect::<Vec<_>>();

        exports.sort_by(|left, right| {
            (left.exported_name.as_str(), left.local_name.as_str())
                .cmp(&(right.exported_name.as_str(), right.local_name.as_str()))
        });

        Ok(FileSemanticIndex {
            language: LanguageId::TypeScript,
            path: self.path.display().to_string(),
            entities: self.entities,
            root_entities,
            exports,
            parser_version: parser_version.to_string(),
        })
    }

    fn collect_module_item(&mut self, item: &ModuleItem) -> Result<(), ParseError> {
        match item {
            ModuleItem::Stmt(statement) => {
                if let swc_ecma_ast::Stmt::Decl(decl) = statement {
                    self.collect_decl(decl, None, false)?;
                }
            }
            ModuleItem::ModuleDecl(decl) => self.collect_module_decl(decl)?,
        }

        Ok(())
    }

    fn collect_module_decl(&mut self, decl: &ModuleDecl) -> Result<(), ParseError> {
        match decl {
            ModuleDecl::ExportDecl(export_decl) => self.collect_export_decl(export_decl)?,
            ModuleDecl::ExportNamed(named_export) => {
                let source = named_export
                    .src
                    .as_ref()
                    .map(|value| value.value.to_string_lossy().into_owned());
                for specifier in &named_export.specifiers {
                    match specifier {
                        ExportSpecifier::Named(named) => {
                            let local = module_export_name_to_string(&named.orig);
                            let exported = named
                                .exported
                                .as_ref()
                                .map(module_export_name_to_string)
                                .unwrap_or_else(|| local.clone());
                            self.export_intents.push(ExportIntent {
                                exported_name: exported,
                                local_name: local,
                                entity_id: None,
                                is_default: false,
                                type_only: named.is_type_only,
                                source: source.clone(),
                            });
                        }
                        ExportSpecifier::Default(default) => {
                            let name = default.exported.sym.to_string();
                            self.export_intents.push(ExportIntent {
                                exported_name: name.clone(),
                                local_name: "default".to_string(),
                                entity_id: None,
                                is_default: true,
                                type_only: false,
                                source: source.clone(),
                            });
                        }
                        ExportSpecifier::Namespace(namespace) => {
                            let name = module_export_name_to_string(&namespace.name);
                            self.export_intents.push(ExportIntent {
                                exported_name: name.clone(),
                                local_name: name,
                                entity_id: None,
                                is_default: false,
                                type_only: false,
                                source: source.clone(),
                            });
                        }
                    }
                }
            }
            ModuleDecl::ExportDefaultDecl(default_decl) => {
                self.collect_export_default_decl(default_decl)?
            }
            ModuleDecl::ExportDefaultExpr(default_expr) => {
                self.collect_export_default_expr(default_expr)?;
            }
            ModuleDecl::ExportAll(export_all) => {
                self.export_intents.push(ExportIntent {
                    exported_name: "*".to_string(),
                    local_name: "*".to_string(),
                    entity_id: None,
                    is_default: false,
                    type_only: export_all.type_only,
                    source: Some(export_all.src.value.to_string_lossy().into_owned()),
                });
            }
            _ => {}
        }

        Ok(())
    }

    fn collect_export_decl(&mut self, export_decl: &ExportDecl) -> Result<(), ParseError> {
        let collected = self.collect_decl(&export_decl.decl, None, true)?;
        for (local_name, entity_id) in collected {
            self.export_intents.push(ExportIntent {
                exported_name: local_name.clone(),
                local_name,
                entity_id: Some(entity_id),
                is_default: false,
                type_only: false,
                source: None,
            });
        }

        Ok(())
    }

    fn collect_export_default_decl(
        &mut self,
        export_default: &ExportDefaultDecl,
    ) -> Result<(), ParseError> {
        match &export_default.decl {
            DefaultDecl::Fn(fn_expr) => {
                let name = fn_expr
                    .ident
                    .as_ref()
                    .map(|ident| ident.sym.to_string())
                    .unwrap_or_else(|| "default".to_string());
                let entity = self.build_function_entity(
                    name.clone(),
                    name.clone(),
                    None,
                    &fn_expr.function,
                    export_default.span,
                    "export_default_function",
                )?;
                let entity_id = entity.id.clone();
                self.push_entity(entity, true);
                self.export_intents.push(ExportIntent {
                    exported_name: "default".to_string(),
                    local_name: name,
                    entity_id: Some(entity_id),
                    is_default: true,
                    type_only: false,
                    source: None,
                });
            }
            DefaultDecl::Class(class_expr) => {
                let name = class_expr
                    .ident
                    .as_ref()
                    .map(|ident| ident.sym.to_string())
                    .unwrap_or_else(|| "default".to_string());
                let class_id = self.collect_class_entity(
                    name.clone(),
                    name.clone(),
                    None,
                    &class_expr.class,
                    export_default.span,
                    true,
                    "export_default_class",
                )?;
                self.export_intents.push(ExportIntent {
                    exported_name: "default".to_string(),
                    local_name: name,
                    entity_id: Some(class_id),
                    is_default: true,
                    type_only: false,
                    source: None,
                });
            }
            DefaultDecl::TsInterfaceDecl(interface_decl) => {
                let name = interface_decl.id.sym.to_string();
                let entity = self.build_interface_entity(
                    &name,
                    &name,
                    None,
                    interface_decl,
                    "export_default_interface",
                )?;
                let entity_id = entity.id.clone();
                self.push_entity(entity, true);
                self.export_intents.push(ExportIntent {
                    exported_name: "default".to_string(),
                    local_name: name,
                    entity_id: Some(entity_id),
                    is_default: true,
                    type_only: true,
                    source: None,
                });
            }
        }

        Ok(())
    }

    fn collect_export_default_expr(
        &mut self,
        export_default: &ExportDefaultExpr,
    ) -> Result<(), ParseError> {
        match export_default.expr.as_ref() {
            Expr::Fn(fn_expr) => {
                let name = fn_expr
                    .ident
                    .as_ref()
                    .map(|ident| ident.sym.to_string())
                    .unwrap_or_else(|| "default".to_string());
                let entity = self.build_function_entity(
                    name.clone(),
                    name.clone(),
                    None,
                    &fn_expr.function,
                    export_default.span,
                    "export_default_function_expr",
                )?;
                let entity_id = entity.id.clone();
                self.push_entity(entity, true);
                self.export_intents.push(ExportIntent {
                    exported_name: "default".to_string(),
                    local_name: name,
                    entity_id: Some(entity_id),
                    is_default: true,
                    type_only: false,
                    source: None,
                });
            }
            Expr::Class(class_expr) => {
                let name = class_expr
                    .ident
                    .as_ref()
                    .map(|ident| ident.sym.to_string())
                    .unwrap_or_else(|| "default".to_string());
                let class_id = self.collect_class_entity(
                    name.clone(),
                    name.clone(),
                    None,
                    &class_expr.class,
                    export_default.span,
                    true,
                    "export_default_class_expr",
                )?;
                self.export_intents.push(ExportIntent {
                    exported_name: "default".to_string(),
                    local_name: name,
                    entity_id: Some(class_id),
                    is_default: true,
                    type_only: false,
                    source: None,
                });
            }
            _ => {
                self.export_intents.push(ExportIntent {
                    exported_name: "default".to_string(),
                    local_name: "default".to_string(),
                    entity_id: None,
                    is_default: true,
                    type_only: false,
                    source: None,
                });
            }
        }

        Ok(())
    }

    fn collect_decl(
        &mut self,
        decl: &Decl,
        parent: Option<&ParentContext>,
        is_root: bool,
    ) -> Result<Vec<(String, String)>, ParseError> {
        let mut collected = Vec::new();

        match decl {
            Decl::Fn(fn_decl) => {
                let name = fn_decl.ident.sym.to_string();
                let path = self.build_entity_path(parent, &name, false);
                let entity = self.build_function_entity(
                    name.clone(),
                    path,
                    parent,
                    &fn_decl.function,
                    fn_decl.function.span,
                    "function_decl",
                )?;
                let entity_id = entity.id.clone();
                self.push_entity(entity, is_root);
                collected.push((name, entity_id));
            }
            Decl::Class(class_decl) => {
                let name = class_decl.ident.sym.to_string();
                let path = self.build_entity_path(parent, &name, false);
                let entity_id = self.collect_class_entity(
                    name.clone(),
                    path,
                    parent,
                    &class_decl.class,
                    class_decl.class.span,
                    is_root,
                    "class_decl",
                )?;
                collected.push((name, entity_id));
            }
            Decl::TsInterface(interface_decl) => {
                let name = interface_decl.id.sym.to_string();
                let path = self.build_entity_path(parent, &name, false);
                let entity = self.build_interface_entity(
                    &name,
                    &path,
                    parent,
                    interface_decl,
                    "interface_decl",
                )?;
                let entity_id = entity.id.clone();
                self.push_entity(entity, is_root);
                collected.push((name, entity_id));
            }
            Decl::TsTypeAlias(type_alias) => {
                let name = type_alias.id.sym.to_string();
                let path = self.build_entity_path(parent, &name, false);
                let entity = self.build_type_alias_entity(
                    &name,
                    &path,
                    parent,
                    type_alias,
                    "type_alias_decl",
                )?;
                let entity_id = entity.id.clone();
                self.push_entity(entity, is_root);
                collected.push((name, entity_id));
            }
            Decl::Var(var_decl) => {
                let entities = self.build_var_entities(var_decl, parent, is_root)?;
                collected.extend(entities);
            }
            _ => {}
        }

        Ok(collected)
    }

    fn build_var_entities(
        &mut self,
        var_decl: &VarDecl,
        parent: Option<&ParentContext>,
        is_root: bool,
    ) -> Result<Vec<(String, String)>, ParseError> {
        let mut collected = Vec::new();

        for declarator in &var_decl.decls {
            let Pat::Ident(binding) = &declarator.name else {
                continue;
            };

            let name = binding.id.sym.to_string();
            let path = self.build_entity_path(parent, &name, false);
            let kind = if matches!(var_decl.kind, VarDeclKind::Const) {
                EntityKind::Constant
            } else {
                EntityKind::Variable
            };

            let signature = self.build_variable_signature(var_decl.kind, binding, declarator)?;
            let signature_shape =
                self.build_variable_signature_shape(var_decl.kind, binding, declarator)?;
            let body_text = declarator
                .init
                .as_deref()
                .map(|expr| self.render_expression_body(expr))
                .transpose()?;
            let canonical_body = body_text
                .as_deref()
                .map(normalize_whitespace)
                .unwrap_or_default();
            let body_hash = hash_string(&canonical_body);
            let snippet = Some(self.span_snippet(declarator.span)?);
            let mut metadata = BTreeMap::new();
            metadata.insert(
                SOURCE_KIND_KEY.to_string(),
                Value::String("variable_decl".to_string()),
            );
            metadata.insert(
                "binding_kind".to_string(),
                Value::String(
                    match var_decl.kind {
                        VarDeclKind::Const => "const",
                        VarDeclKind::Let => "let",
                        VarDeclKind::Var => "var",
                    }
                    .to_string(),
                ),
            );
            if matches!(
                declarator.init.as_deref(),
                Some(Expr::Arrow(_) | Expr::Fn(_))
            ) {
                metadata.insert("callable".to_string(), Value::Bool(true));
            }

            let dependencies = declarator
                .init
                .as_deref()
                .map(collect_dependencies)
                .transpose()?
                .unwrap_or_default();

            let entity = self.build_entity(
                kind,
                &name,
                &path,
                parent,
                declarator.span,
                signature,
                signature_shape,
                body_hash,
                snippet,
                body_text,
                dependencies,
                Vec::new(),
                metadata,
            );
            let entity_id = entity.id.clone();
            self.push_entity(entity, is_root);
            collected.push((name, entity_id));
        }

        Ok(collected)
    }

    fn build_function_entity(
        &self,
        name: String,
        path: String,
        parent: Option<&ParentContext>,
        function: &Function,
        span: Span,
        source_kind: &str,
    ) -> Result<SemanticEntity, ParseError> {
        let signature = self.build_function_signature(&name, function)?;
        let signature_shape = self.build_function_signature_shape(function)?;
        let body = function
            .body
            .as_ref()
            .map(|body| self.emit_node(body))
            .transpose()?;
        let body_hash = hash_optional_text(body.as_deref());
        let dependencies = collect_dependencies(function)?;
        let snippet = Some(self.span_snippet(span)?);
        let mut metadata = BTreeMap::new();
        metadata.insert(
            SOURCE_KIND_KEY.to_string(),
            Value::String(source_kind.to_string()),
        );

        Ok(self.build_entity(
            EntityKind::Function,
            &name,
            &path,
            parent,
            span,
            signature,
            signature_shape,
            body_hash,
            snippet,
            body,
            dependencies,
            Vec::new(),
            metadata,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn collect_class_entity(
        &mut self,
        name: String,
        path: String,
        parent: Option<&ParentContext>,
        class: &Class,
        span: Span,
        is_root: bool,
        source_kind: &str,
    ) -> Result<String, ParseError> {
        let signature = self.build_class_signature(&name, class)?;
        let signature_shape = self.build_class_signature_shape(class)?;
        let class_parent_anchor = self.parent_anchor(parent);
        let class_id = build_entity_id(
            EntityKind::Class,
            &class_parent_anchor,
            &name,
            &normalize_whitespace(&signature_shape),
        );
        let class_parent = parent.map(|value| value.id.clone());
        let parent_context = ParentContext {
            id: class_id.clone(),
            path: path.clone(),
        };

        let mut child_entities = Vec::new();
        let mut child_ids = Vec::new();
        let mut non_method_parts = Vec::new();
        let mut dependency_collector = DependencyCollector::default();

        if let Some(super_class) = class.super_class.as_deref() {
            super_class.visit_with(&mut dependency_collector);
        }
        for implement in &class.implements {
            implement.visit_with(&mut dependency_collector);
        }

        for member in &class.body {
            match member {
                ClassMember::Constructor(constructor) => {
                    let entity =
                        self.build_constructor_entity(&parent_context, constructor, "constructor")?;
                    child_ids.push(entity.id.clone());
                    child_entities.push(entity);
                }
                ClassMember::Method(method) => {
                    let entity =
                        self.build_method_entity(&parent_context, method, false, "class_method")?;
                    child_ids.push(entity.id.clone());
                    child_entities.push(entity);
                }
                ClassMember::PrivateMethod(method) => {
                    let entity = self.build_private_method_entity(
                        &parent_context,
                        method,
                        "private_method",
                    )?;
                    child_ids.push(entity.id.clone());
                    child_entities.push(entity);
                }
                other => {
                    non_method_parts.push(self.emit_node(other)?);
                    other.visit_with(&mut dependency_collector);
                }
            }
        }

        let body_text = (!non_method_parts.is_empty()).then(|| non_method_parts.join("\n"));
        let body_hash = hash_optional_text(body_text.as_deref());
        let snippet = Some(self.span_snippet(span)?);
        let mut metadata = BTreeMap::new();
        metadata.insert(
            SOURCE_KIND_KEY.to_string(),
            Value::String(source_kind.to_string()),
        );
        metadata.insert(
            "child_method_count".to_string(),
            Value::from(child_ids.len() as u64),
        );
        let dependencies = dependency_collector.finish(Some(&name));

        let class_entity = self.build_entity_with_anchor(
            EntityKind::Class,
            &name,
            &path,
            class_parent,
            &class_parent_anchor,
            span,
            signature,
            signature_shape,
            body_hash,
            snippet,
            body_text,
            dependencies,
            child_ids,
            metadata,
        );

        self.push_entity(class_entity, is_root);
        for child in child_entities {
            self.push_entity(child, false);
        }

        Ok(class_id)
    }

    fn build_constructor_entity(
        &self,
        parent: &ParentContext,
        constructor: &Constructor,
        source_kind: &str,
    ) -> Result<SemanticEntity, ParseError> {
        let name = "constructor".to_string();
        let path = format!("{}#constructor", parent.path);
        let signature = self.build_constructor_signature(constructor)?;
        let signature_shape = self.build_constructor_signature_shape(constructor)?;
        let body = constructor
            .body
            .as_ref()
            .map(|body| self.emit_node(body))
            .transpose()?;
        let body_hash = hash_optional_text(body.as_deref());
        let snippet = Some(self.span_snippet(constructor.span)?);
        let mut metadata = BTreeMap::new();
        metadata.insert(
            SOURCE_KIND_KEY.to_string(),
            Value::String(source_kind.to_string()),
        );
        let dependencies = collect_dependencies(constructor)?;

        Ok(self.build_entity_with_anchor(
            EntityKind::Method,
            &name,
            &path,
            Some(parent.id.clone()),
            &parent.path,
            constructor.span,
            signature,
            signature_shape,
            body_hash,
            snippet,
            body,
            dependencies,
            Vec::new(),
            metadata,
        ))
    }

    fn build_method_entity(
        &self,
        parent: &ParentContext,
        method: &ClassMethod,
        is_private: bool,
        source_kind: &str,
    ) -> Result<SemanticEntity, ParseError> {
        let name = prop_name_to_string(&method.key, &self.cm)?;
        let path = format!(
            "{}{}{}",
            parent.path,
            if method.is_static { "." } else { "#" },
            name
        );
        let signature = self.build_method_signature(&name, method)?;
        let signature_shape = self.build_method_signature_shape(method)?;
        let body = method
            .function
            .body
            .as_ref()
            .map(|body| self.emit_node(body))
            .transpose()?;
        let body_hash = hash_optional_text(body.as_deref());
        let snippet = Some(self.span_snippet(method.span)?);
        let mut metadata = BTreeMap::new();
        metadata.insert(
            SOURCE_KIND_KEY.to_string(),
            Value::String(source_kind.to_string()),
        );
        metadata.insert("is_private".to_string(), Value::Bool(is_private));
        metadata.insert(
            "method_kind".to_string(),
            Value::String(
                match method.kind {
                    MethodKind::Method => "method",
                    MethodKind::Getter => "getter",
                    MethodKind::Setter => "setter",
                }
                .to_string(),
            ),
        );
        let dependencies = collect_dependencies(&*method.function)?;

        Ok(self.build_entity_with_anchor(
            EntityKind::Method,
            &name,
            &path,
            Some(parent.id.clone()),
            &parent.path,
            method.span,
            signature,
            signature_shape,
            body_hash,
            snippet,
            body,
            dependencies,
            Vec::new(),
            metadata,
        ))
    }

    fn build_private_method_entity(
        &self,
        parent: &ParentContext,
        method: &PrivateMethod,
        source_kind: &str,
    ) -> Result<SemanticEntity, ParseError> {
        let name = format!("#{}", method.key.name);
        let path = format!(
            "{}{}{}",
            parent.path,
            if method.is_static { "." } else { "#" },
            name
        );
        let signature = self.build_private_method_signature(&name, method)?;
        let signature_shape = self.build_private_method_signature_shape(method)?;
        let body = method
            .function
            .body
            .as_ref()
            .map(|body| self.emit_node(body))
            .transpose()?;
        let body_hash = hash_optional_text(body.as_deref());
        let snippet = Some(self.span_snippet(method.span)?);
        let mut metadata = BTreeMap::new();
        metadata.insert(
            SOURCE_KIND_KEY.to_string(),
            Value::String(source_kind.to_string()),
        );
        metadata.insert("is_private".to_string(), Value::Bool(true));
        let dependencies = collect_dependencies(&*method.function)?;

        Ok(self.build_entity_with_anchor(
            EntityKind::Method,
            &name,
            &path,
            Some(parent.id.clone()),
            &parent.path,
            method.span,
            signature,
            signature_shape,
            body_hash,
            snippet,
            body,
            dependencies,
            Vec::new(),
            metadata,
        ))
    }

    fn build_interface_entity(
        &self,
        name: &str,
        path: &str,
        parent: Option<&ParentContext>,
        interface_decl: &TsInterfaceDecl,
        source_kind: &str,
    ) -> Result<SemanticEntity, ParseError> {
        let signature = self.build_interface_signature(name, interface_decl)?;
        let signature_shape = self.build_interface_signature_shape(interface_decl)?;
        let body = Some(self.emit_node(&interface_decl.body)?);
        let body_hash = hash_optional_text(body.as_deref());
        let snippet = Some(self.span_snippet(interface_decl.span)?);
        let mut metadata = BTreeMap::new();
        metadata.insert(
            SOURCE_KIND_KEY.to_string(),
            Value::String(source_kind.to_string()),
        );
        let dependencies = collect_dependencies(interface_decl)?;

        Ok(self.build_entity(
            EntityKind::Interface,
            name,
            path,
            parent,
            interface_decl.span,
            signature,
            signature_shape,
            body_hash,
            snippet,
            body,
            dependencies,
            Vec::new(),
            metadata,
        ))
    }

    fn build_type_alias_entity(
        &self,
        name: &str,
        path: &str,
        parent: Option<&ParentContext>,
        type_alias: &TsTypeAliasDecl,
        source_kind: &str,
    ) -> Result<SemanticEntity, ParseError> {
        let signature = self.build_type_alias_signature(name, type_alias)?;
        let signature_shape = self.build_type_alias_signature_shape(type_alias)?;
        let body = Some(self.emit_node(&*type_alias.type_ann)?);
        let body_hash = hash_optional_text(body.as_deref());
        let snippet = Some(self.span_snippet(type_alias.span)?);
        let mut metadata = BTreeMap::new();
        metadata.insert(
            SOURCE_KIND_KEY.to_string(),
            Value::String(source_kind.to_string()),
        );
        let dependencies = collect_dependencies(type_alias)?;

        Ok(self.build_entity(
            EntityKind::TypeAlias,
            name,
            path,
            parent,
            type_alias.span,
            signature,
            signature_shape,
            body_hash,
            snippet,
            body,
            dependencies,
            Vec::new(),
            metadata,
        ))
    }

    #[allow(clippy::too_many_arguments)]
    fn build_entity(
        &self,
        kind: EntityKind,
        name: &str,
        path: &str,
        parent: Option<&ParentContext>,
        span: Span,
        signature: String,
        signature_shape: String,
        body_hash: String,
        snippet: Option<String>,
        body: Option<String>,
        dependencies: Vec<String>,
        children: Vec<String>,
        metadata: BTreeMap<String, Value>,
    ) -> SemanticEntity {
        self.build_entity_with_anchor(
            kind,
            name,
            path,
            parent.map(|value| value.id.clone()),
            &self.parent_anchor(parent),
            span,
            signature,
            signature_shape,
            body_hash,
            snippet,
            body,
            dependencies,
            children,
            metadata,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn build_entity_with_anchor(
        &self,
        kind: EntityKind,
        name: &str,
        path: &str,
        parent: Option<String>,
        parent_anchor: &str,
        span: Span,
        signature: String,
        signature_shape: String,
        body_hash: String,
        snippet: Option<String>,
        body: Option<String>,
        mut dependencies: Vec<String>,
        children: Vec<String>,
        mut metadata: BTreeMap<String, Value>,
    ) -> SemanticEntity {
        dependencies.sort();
        dependencies.dedup();

        let normalized_signature = normalize_whitespace(&signature);
        let normalized_shape = normalize_whitespace(&signature_shape);
        let signature_hash = hash_string(&normalized_signature);
        let id = build_entity_id(kind, parent_anchor, name, &normalized_shape);
        let shape_hash = build_shape_hash(kind, parent_anchor, &normalized_shape);
        let full_hash = build_full_hash(kind, &signature_hash, &body_hash);

        metadata.insert(
            SIGNATURE_SHAPE_KEY.to_string(),
            Value::String(signature_shape),
        );
        metadata.insert(
            PARENT_ANCHOR_KEY.to_string(),
            Value::String(parent_anchor.to_string()),
        );

        SemanticEntity {
            id,
            kind,
            name: name.to_string(),
            signature,
            path: path.to_string(),
            parent,
            body_hash,
            full_hash,
            signature_hash,
            shape_hash,
            span: self.span_range(span),
            semantic_span: self.span_range(span),
            dependencies,
            children,
            metadata,
            snippet,
            body,
        }
    }

    fn push_entity(&mut self, entity: SemanticEntity, is_root: bool) {
        if is_root {
            self.name_to_id
                .entry(entity.name.clone())
                .or_insert_with(|| entity.id.clone());
        }
        self.entities.push(entity);
    }

    fn build_entity_path(
        &self,
        parent: Option<&ParentContext>,
        name: &str,
        is_static: bool,
    ) -> String {
        match parent {
            Some(parent) => format!(
                "{}{}{}",
                parent.path,
                if is_static { "." } else { "#" },
                name
            ),
            None => name.to_string(),
        }
    }

    fn parent_anchor(&self, parent: Option<&ParentContext>) -> String {
        parent
            .map(|value| value.path.clone())
            .unwrap_or_else(|| MODULE_ANCHOR.to_string())
    }

    fn build_function_signature(
        &self,
        name: &str,
        function: &Function,
    ) -> Result<String, ParseError> {
        Ok(format!(
            "{}function{} {}{}({}){}",
            if function.is_async { "async " } else { "" },
            if function.is_generator { "*" } else { "" },
            name,
            self.render_type_params(function.type_params.as_deref())?,
            self.render_params(&function.params)?,
            self.render_return_type(function.return_type.as_deref())?,
        ))
    }

    fn build_function_signature_shape(&self, function: &Function) -> Result<String, ParseError> {
        Ok(format!(
            "{}function{} __NAME__{}({}){}",
            if function.is_async { "async " } else { "" },
            if function.is_generator { "*" } else { "" },
            self.render_type_params(function.type_params.as_deref())?,
            self.render_params(&function.params)?,
            self.render_return_type(function.return_type.as_deref())?,
        ))
    }

    fn build_constructor_signature(&self, constructor: &Constructor) -> Result<String, ParseError> {
        Ok(format!(
            "{}constructor({})",
            render_accessibility(constructor.accessibility),
            self.render_constructor_params(&constructor.params)?,
        ))
    }

    fn build_constructor_signature_shape(
        &self,
        constructor: &Constructor,
    ) -> Result<String, ParseError> {
        Ok(format!(
            "{}constructor({})",
            render_accessibility(constructor.accessibility),
            self.render_constructor_params(&constructor.params)?,
        ))
    }

    fn build_method_signature(
        &self,
        name: &str,
        method: &ClassMethod,
    ) -> Result<String, ParseError> {
        Ok(format!(
            "{}{}{}{}{}{}({}){}",
            render_accessibility(method.accessibility),
            if method.is_static { "static " } else { "" },
            if method.function.is_async {
                "async "
            } else {
                ""
            },
            render_method_kind_prefix(method.kind),
            name,
            self.render_type_params(method.function.type_params.as_deref())?,
            self.render_params(&method.function.params)?,
            self.render_return_type(method.function.return_type.as_deref())?,
        ))
    }

    fn build_method_signature_shape(&self, method: &ClassMethod) -> Result<String, ParseError> {
        Ok(format!(
            "{}{}{}{}__NAME__{}({}){}",
            render_accessibility(method.accessibility),
            if method.is_static { "static " } else { "" },
            if method.function.is_async {
                "async "
            } else {
                ""
            },
            render_method_kind_prefix(method.kind),
            self.render_type_params(method.function.type_params.as_deref())?,
            self.render_params(&method.function.params)?,
            self.render_return_type(method.function.return_type.as_deref())?,
        ))
    }

    fn build_private_method_signature(
        &self,
        name: &str,
        method: &PrivateMethod,
    ) -> Result<String, ParseError> {
        Ok(format!(
            "{}{}{}{}{}{}({}){}",
            "",
            if method.is_static { "static " } else { "" },
            if method.function.is_async {
                "async "
            } else {
                ""
            },
            render_method_kind_prefix(method.kind),
            name,
            self.render_type_params(method.function.type_params.as_deref())?,
            self.render_params(&method.function.params)?,
            self.render_return_type(method.function.return_type.as_deref())?,
        ))
    }

    fn build_private_method_signature_shape(
        &self,
        method: &PrivateMethod,
    ) -> Result<String, ParseError> {
        Ok(format!(
            "{}{}{}__NAME__{}({}){}",
            if method.is_static { "static " } else { "" },
            if method.function.is_async {
                "async "
            } else {
                ""
            },
            render_method_kind_prefix(method.kind),
            self.render_type_params(method.function.type_params.as_deref())?,
            self.render_params(&method.function.params)?,
            self.render_return_type(method.function.return_type.as_deref())?,
        ))
    }

    fn build_class_signature(&self, name: &str, class: &Class) -> Result<String, ParseError> {
        let extends = if let Some(super_class) = class.super_class.as_deref() {
            let mut rendered = format!(" extends {}", self.emit_node(super_class)?);
            if let Some(type_args) = class.super_type_params.as_deref() {
                rendered.push_str(&self.emit_node(type_args)?);
            }
            rendered
        } else {
            String::new()
        };
        let implements = if class.implements.is_empty() {
            String::new()
        } else {
            format!(
                " implements {}",
                class
                    .implements
                    .iter()
                    .map(|value| self.emit_node(value))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            )
        };

        Ok(format!(
            "{}class {}{}{}{}",
            if class.is_abstract { "abstract " } else { "" },
            name,
            self.render_type_params(class.type_params.as_deref())?,
            extends,
            implements,
        ))
    }

    fn build_class_signature_shape(&self, class: &Class) -> Result<String, ParseError> {
        let extends = if let Some(super_class) = class.super_class.as_deref() {
            let mut rendered = format!(" extends {}", self.emit_node(super_class)?);
            if let Some(type_args) = class.super_type_params.as_deref() {
                rendered.push_str(&self.emit_node(type_args)?);
            }
            rendered
        } else {
            String::new()
        };
        let implements = if class.implements.is_empty() {
            String::new()
        } else {
            format!(
                " implements {}",
                class
                    .implements
                    .iter()
                    .map(|value| self.emit_node(value))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            )
        };

        Ok(format!(
            "{}class __NAME__{}{}{}",
            if class.is_abstract { "abstract " } else { "" },
            self.render_type_params(class.type_params.as_deref())?,
            extends,
            implements,
        ))
    }

    fn build_interface_signature(
        &self,
        name: &str,
        interface_decl: &TsInterfaceDecl,
    ) -> Result<String, ParseError> {
        let extends = if interface_decl.extends.is_empty() {
            String::new()
        } else {
            format!(
                " extends {}",
                interface_decl
                    .extends
                    .iter()
                    .map(|value| self.emit_node(value))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            )
        };

        Ok(format!(
            "interface {}{}{}",
            name,
            self.render_type_params(interface_decl.type_params.as_deref())?,
            extends,
        ))
    }

    fn build_interface_signature_shape(
        &self,
        interface_decl: &TsInterfaceDecl,
    ) -> Result<String, ParseError> {
        let extends = if interface_decl.extends.is_empty() {
            String::new()
        } else {
            format!(
                " extends {}",
                interface_decl
                    .extends
                    .iter()
                    .map(|value| self.emit_node(value))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            )
        };

        Ok(format!(
            "interface __NAME__{}{}",
            self.render_type_params(interface_decl.type_params.as_deref())?,
            extends,
        ))
    }

    fn build_type_alias_signature(
        &self,
        name: &str,
        type_alias: &TsTypeAliasDecl,
    ) -> Result<String, ParseError> {
        Ok(format!(
            "type {}{}={}",
            name,
            self.render_type_params(type_alias.type_params.as_deref())?,
            self.emit_node(&*type_alias.type_ann)?,
        ))
    }

    fn build_type_alias_signature_shape(
        &self,
        type_alias: &TsTypeAliasDecl,
    ) -> Result<String, ParseError> {
        Ok(format!(
            "type __NAME__{}={}",
            self.render_type_params(type_alias.type_params.as_deref())?,
            self.emit_node(&*type_alias.type_ann)?,
        ))
    }

    fn build_variable_signature(
        &self,
        kind: VarDeclKind,
        binding: &BindingIdent,
        declarator: &VarDeclarator,
    ) -> Result<String, ParseError> {
        let annotation = binding
            .type_ann
            .as_deref()
            .map(|type_ann| self.emit_node(type_ann))
            .transpose()?
            .unwrap_or_default();
        let initializer = declarator
            .init
            .as_deref()
            .map(|value| self.render_initializer_signature(value))
            .transpose()?
            .unwrap_or_default();

        Ok(format!(
            "{} {}{}{}",
            render_var_decl_kind(kind),
            binding.id.sym,
            annotation,
            initializer,
        ))
    }

    fn build_variable_signature_shape(
        &self,
        kind: VarDeclKind,
        binding: &BindingIdent,
        declarator: &VarDeclarator,
    ) -> Result<String, ParseError> {
        let annotation = binding
            .type_ann
            .as_deref()
            .map(|type_ann| self.emit_node(type_ann))
            .transpose()?
            .unwrap_or_default();
        let initializer = declarator
            .init
            .as_deref()
            .map(|value| self.render_initializer_signature_shape(value))
            .transpose()?
            .unwrap_or_default();

        Ok(format!(
            "{} __NAME__{}{}",
            render_var_decl_kind(kind),
            annotation,
            initializer,
        ))
    }

    fn render_params(&self, params: &[Param]) -> Result<String, ParseError> {
        params
            .iter()
            .map(|param| self.emit_node(&param.pat))
            .collect::<Result<Vec<_>, _>>()
            .map(|values| values.join(", "))
    }

    fn render_constructor_params(
        &self,
        params: &[ParamOrTsParamProp],
    ) -> Result<String, ParseError> {
        params
            .iter()
            .map(|param| self.emit_node(param))
            .collect::<Result<Vec<_>, _>>()
            .map(|values| values.join(", "))
    }

    fn render_type_params<T>(&self, type_params: Option<&T>) -> Result<String, ParseError>
    where
        T: Node,
    {
        type_params
            .map(|value| self.emit_node(value))
            .transpose()
            .map(|value| value.unwrap_or_default())
    }

    fn render_return_type<T>(&self, return_type: Option<&T>) -> Result<String, ParseError>
    where
        T: Node,
    {
        return_type
            .map(|value| self.emit_node(value))
            .transpose()
            .map(|value| value.unwrap_or_default())
    }

    fn render_expression_body(&self, expr: &Expr) -> Result<String, ParseError> {
        match expr {
            Expr::Arrow(arrow) => self.render_arrow_body(arrow),
            Expr::Fn(fn_expr) => fn_expr
                .function
                .body
                .as_ref()
                .map(|body| self.emit_node(body))
                .transpose()
                .map(|value| value.unwrap_or_default()),
            other => self.emit_node(other),
        }
    }

    fn render_arrow_body(&self, arrow: &ArrowExpr) -> Result<String, ParseError> {
        match arrow.body.as_ref() {
            BlockStmtOrExpr::BlockStmt(body) => self.emit_node(body),
            BlockStmtOrExpr::Expr(expr) => self.emit_node(expr.as_ref()),
        }
    }

    fn render_initializer_signature(&self, expr: &Expr) -> Result<String, ParseError> {
        match expr {
            Expr::Fn(fn_expr) => Ok(format!(
                "= {}",
                self.build_function_signature("function", &fn_expr.function,)?
                    .replace(" function", "")
            )),
            Expr::Arrow(arrow) => Ok(format!(
                "= {}{}({}){} => {}",
                if arrow.is_async { "async " } else { "" },
                self.render_type_params(arrow.type_params.as_deref())?,
                arrow
                    .params
                    .iter()
                    .map(|param| self.emit_node(param))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", "),
                self.render_return_type(arrow.return_type.as_deref())?,
                self.render_arrow_body(arrow)?,
            )),
            other => Ok(format!(" = {}", self.emit_node(other)?)),
        }
    }

    fn render_initializer_signature_shape(&self, expr: &Expr) -> Result<String, ParseError> {
        match expr {
            Expr::Fn(fn_expr) => Ok(format!(
                "= {}",
                self.build_function_signature_shape(&fn_expr.function)?
                    .replace("__NAME__", "function")
                    .replace(" function", "")
            )),
            Expr::Arrow(arrow) => Ok(format!(
                "= {}{}({}){} => {}",
                if arrow.is_async { "async " } else { "" },
                self.render_type_params(arrow.type_params.as_deref())?,
                arrow
                    .params
                    .iter()
                    .map(|param| self.emit_node(param))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", "),
                self.render_return_type(arrow.return_type.as_deref())?,
                match arrow.body.as_ref() {
                    BlockStmtOrExpr::BlockStmt(_) => "{...}".to_string(),
                    BlockStmtOrExpr::Expr(_) => "__EXPR__".to_string(),
                },
            )),
            other => Ok(format!(" = {}", self.emit_node(other)?)),
        }
    }

    fn span_range(&self, span: Span) -> SpanRange {
        SpanRange {
            start: self.span_location(span.lo),
            end: self.span_location(span.hi),
        }
    }

    fn span_location(&self, position: BytePos) -> SpanLocation {
        let location = self.cm.lookup_char_pos(position);
        SpanLocation {
            byte: position.0.saturating_sub(1),
            line: location.line,
            column: location.col_display + 1,
        }
    }

    fn span_snippet(&self, span: Span) -> Result<String, ParseError> {
        if let Ok(snippet) = self.cm.span_to_snippet(span) {
            return Ok(snippet);
        }

        let start = span.lo.0.saturating_sub(1) as usize;
        let end = span.hi.0.saturating_sub(1) as usize;
        self.source
            .get(start..end)
            .map(ToOwned::to_owned)
            .ok_or_else(|| ParseError::InvalidSource {
                path: self.path.display().to_string(),
                message: format!("failed to extract snippet for span {:?}", span),
            })
    }

    fn emit_node<T>(&self, node: &T) -> Result<String, ParseError>
    where
        T: Node,
    {
        let mut output = Vec::new();
        let mut emitter = Emitter {
            cfg: CodegenConfig::default().with_minify(true),
            cm: self.cm.clone(),
            comments: None,
            wr: JsWriter::new(self.cm.clone(), "\n", &mut output, None),
        };

        node.emit_with(&mut emitter)
            .map_err(|error| ParseError::InvalidSource {
                path: self.path.display().to_string(),
                message: format!("failed to render AST node: {error}"),
            })?;

        String::from_utf8(output).map_err(|error| ParseError::InvalidSource {
            path: self.path.display().to_string(),
            message: format!("codegen produced invalid utf-8: {error}"),
        })
    }
}

#[derive(Default)]
struct DependencyCollector {
    identifiers: BTreeSet<String>,
}

impl DependencyCollector {
    fn finish(mut self, current_name: Option<&str>) -> Vec<String> {
        if let Some(name) = current_name {
            self.identifiers.remove(name);
        }
        self.identifiers.into_iter().collect::<Vec<_>>()
    }
}

impl Visit for DependencyCollector {
    fn visit_binding_ident(&mut self, _node: &BindingIdent) {}

    fn visit_ident(&mut self, node: &Ident) {
        self.identifiers.insert(node.sym.to_string());
    }

    fn visit_ident_name(&mut self, _node: &IdentName) {}

    fn visit_private_name(&mut self, _node: &PrivateName) {}

    fn visit_prop_name(&mut self, node: &PropName) {
        if let PropName::Computed(computed) = node {
            computed.visit_with(self);
        }
    }
}

fn collect_dependencies<T>(node: &T) -> Result<Vec<String>, ParseError>
where
    T: VisitWith<DependencyCollector>,
{
    let mut collector = DependencyCollector::default();
    node.visit_with(&mut collector);
    Ok(collector.finish(None))
}

fn prop_name_to_string(prop_name: &PropName, cm: &Lrc<SourceMap>) -> Result<String, ParseError> {
    match prop_name {
        PropName::Ident(value) => Ok(value.sym.to_string()),
        PropName::Str(value) => Ok(value.value.to_string_lossy().into_owned()),
        PropName::Num(value) => Ok(value.value.to_string()),
        PropName::BigInt(value) => Ok(value.value.to_string()),
        PropName::Computed(value) => {
            let mut output = Vec::new();
            let mut emitter = Emitter {
                cfg: CodegenConfig::default().with_minify(true),
                cm: cm.clone(),
                comments: None,
                wr: JsWriter::new(cm.clone(), "\n", &mut output, None),
            };
            value
                .emit_with(&mut emitter)
                .map_err(|error| ParseError::InvalidSource {
                    path: "<computed-prop>".to_string(),
                    message: format!("failed to render property name: {error}"),
                })?;
            String::from_utf8(output)
                .map(|value| value.trim().to_string())
                .map_err(|error| ParseError::InvalidSource {
                    path: "<computed-prop>".to_string(),
                    message: format!("computed property name is not utf-8: {error}"),
                })
        }
    }
}

fn module_export_name_to_string(name: &ModuleExportName) -> String {
    match name {
        ModuleExportName::Ident(value) => value.sym.to_string(),
        ModuleExportName::Str(value) => value.value.to_string_lossy().into_owned(),
    }
}

fn render_accessibility(accessibility: Option<swc_ecma_ast::Accessibility>) -> &'static str {
    match accessibility {
        Some(swc_ecma_ast::Accessibility::Public) => "public ",
        Some(swc_ecma_ast::Accessibility::Protected) => "protected ",
        Some(swc_ecma_ast::Accessibility::Private) => "private ",
        None => "",
    }
}

fn render_method_kind_prefix(kind: MethodKind) -> &'static str {
    match kind {
        MethodKind::Method => "",
        MethodKind::Getter => "get ",
        MethodKind::Setter => "set ",
    }
}

fn render_var_decl_kind(kind: VarDeclKind) -> &'static str {
    match kind {
        VarDeclKind::Const => "const",
        VarDeclKind::Let => "let",
        VarDeclKind::Var => "var",
    }
}

fn hash_optional_text(value: Option<&str>) -> String {
    hash_string(value.map(normalize_whitespace).unwrap_or_default())
}
