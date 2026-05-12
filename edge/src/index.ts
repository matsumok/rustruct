import { swaggerUI } from "@hono/swagger-ui";
import { createRoute, OpenAPIHono, z } from "@hono/zod-openapi";
import { createAuth } from "./auth";

const app = new OpenAPIHono<{ Bindings: CloudflareBindings }>();

app.all("/api/auth/*", (c) => {
  const auth = createAuth(c.env);
  return auth.handler(c.req.raw);
});

app.get("/", (c) => {
  return c.text("Hello Hono!");
});

const healthRoute = createRoute({
  method: "get",
  path: "/api/health",
  responses: {
    200: {
      content: {
        "application/json": {
          schema: z.object({ status: z.string() }),
        },
      },
      description: "OK",
    },
  },
});

app.openapi(healthRoute, (c) => {
  return c.json({ status: "ok" });
});

app.doc("/api/doc", {
  openapi: "3.0.0",
  info: {
    title: "Rustruct API",
    version: "0.1.0",
  },
});

app.get("/api/ui", swaggerUI({ url: "/api/doc" }));

export default app;
