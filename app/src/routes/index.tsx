import { createFileRoute } from "@tanstack/react-router";
import { Button } from "@/components/ui/button";

export const Route = createFileRoute("/")({
  component: () => (
    <div>
      <h1> Hello Rustruct!</h1>
      <Button variant="outline">Click me</Button>
    </div>
  ),
});
