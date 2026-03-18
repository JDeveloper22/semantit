import { constants as fsConstants } from "node:fs";
import { access, mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { spawn } from "node:child_process";

import { NextRequest, NextResponse } from "next/server";

import type { PlaygroundOperation } from "@/lib/types";

export const runtime = "nodejs";

type ParseTarget = "left" | "right" | "base";

interface PlaygroundRequest {
  operation: PlaygroundOperation;
  left?: string;
  right?: string;
  base?: string;
  target?: ParseTarget;
}

const repoRoot = path.resolve(process.cwd(), "..");
const cargoManifest = path.join(repoRoot, "Cargo.toml");
const cliBinary = process.env.SEMANTIT_CLI_BIN
  ? path.resolve(process.env.SEMANTIT_CLI_BIN)
  : path.join(
      repoRoot,
      "target",
      "debug",
      process.platform === "win32" ? "semantit-cli.exe" : "semantit-cli",
    );

function runProcess(
  command: string,
  args: string[],
  cwd: string,
): Promise<{ stdout: string; stderr: string }> {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      cwd,
      env: process.env,
      stdio: ["ignore", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";

    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString();
    });

    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString();
    });

    child.on("error", reject);
    child.on("close", (code) => {
      if (code === 0) {
        resolve({ stdout, stderr });
        return;
      }

      reject(
        new Error(
          `Process exited with code ${code}: ${stderr || stdout || "no output"}`,
        ),
      );
    });
  });
}

async function pathExists(filePath: string): Promise<boolean> {
  try {
    await access(filePath, fsConstants.F_OK);
    return true;
  } catch {
    return false;
  }
}

async function ensureCliBinary(): Promise<string> {
  if (await pathExists(cliBinary)) {
    return cliBinary;
  }

  await runProcess(
    "cargo",
    ["build", "-p", "semantit-cli", "--manifest-path", cargoManifest],
    repoRoot,
  );

  if (!(await pathExists(cliBinary))) {
    throw new Error("Semantit CLI was not produced after cargo build.");
  }

  return cliBinary;
}

function assertSource(value: string | undefined, label: string): string {
  if (!value || !value.trim()) {
    throw new Error(`Missing source for ${label}.`);
  }

  return value;
}

async function writeVersion(
  tempDir: string,
  fileName: string,
  source: string,
): Promise<string> {
  const filePath = path.join(tempDir, fileName);
  await writeFile(filePath, source, "utf8");
  return filePath;
}

function jsonError(message: string, status = 400) {
  return NextResponse.json({ error: message }, { status });
}

export async function POST(request: NextRequest) {
  let body: PlaygroundRequest;

  try {
    body = (await request.json()) as PlaygroundRequest;
  } catch {
    return jsonError("Request body must be valid JSON.");
  }

  if (!body.operation) {
    return jsonError("Missing operation.");
  }

  const tempDir = await mkdtemp(path.join(os.tmpdir(), "semantit-playground-"));
  const startedAt = Date.now();

  try {
    const binary = await ensureCliBinary();
    const args: string[] = [];

    if (body.operation === "parse") {
      const target = body.target ?? "left";
      const sourceMap: Record<ParseTarget, string | undefined> = {
        left: body.left,
        right: body.right,
        base: body.base,
      };
      const source = assertSource(sourceMap[target], target);
      const filePath = await writeVersion(tempDir, `${target}.ts`, source);
      args.push("parse", filePath, "--json");
    } else if (body.operation === "diff") {
      const left = await writeVersion(tempDir, "left.ts", assertSource(body.left, "left"));
      const right = await writeVersion(
        tempDir,
        "right.ts",
        assertSource(body.right, "right"),
      );
      args.push("diff", left, right, "--json");
    } else if (body.operation === "merge") {
      const base = await writeVersion(tempDir, "base.ts", assertSource(body.base, "base"));
      const ours = await writeVersion(tempDir, "ours.ts", assertSource(body.left, "left"));
      const theirs = await writeVersion(
        tempDir,
        "theirs.ts",
        assertSource(body.right, "right"),
      );
      args.push("merge", base, ours, theirs, "--json");
    } else {
      return jsonError(`Unsupported operation: ${String(body.operation)}`);
    }

    const { stdout, stderr } = await runProcess(binary, args, repoRoot);

    return NextResponse.json({
      operation: body.operation,
      command: [path.relative(repoRoot, binary), ...args],
      durationMs: Date.now() - startedAt,
      stderr: stderr.trim(),
      result: JSON.parse(stdout),
    });
  } catch (error) {
    const message =
      error instanceof Error ? error.message : "Unknown Semantit playground error.";
    return jsonError(message, 500);
  } finally {
    await rm(tempDir, { recursive: true, force: true });
  }
}
