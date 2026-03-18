import { SemanticWorkbench } from "@/components/semantic-workbench";
import { loadDemoScenarios } from "@/lib/demo-scenarios";

export default async function Home() {
  const scenarios = await loadDemoScenarios();

  return <SemanticWorkbench scenarios={scenarios} />;
}
