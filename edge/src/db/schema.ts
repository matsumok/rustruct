import { integer, sqliteTable, text } from "drizzle-orm/sqlite-core";

export const projects = sqliteTable("projects", {
  id: text("id").primaryKey(),
  userId: text("user_id").notNull(),
  name: text("name").notNull(),
  createdAt: integer("created_at", { mode: "timestamp" }).notNull(),
});

export const items = sqliteTable("items", {
  id: text("id").primaryKey(),
  userId: text("user_id").notNull(),
  projectId: text("project_id"),
  type: text("type").notNull(),
  name: text("name").notNull(),
  data: text("data"),
  r2Key: text("r2_key"),
  status: text("status", { enum: ["pending", "done", "error"] }),
  createdAt: integer("created_at", { mode: "timestamp" }).notNull(),
});
