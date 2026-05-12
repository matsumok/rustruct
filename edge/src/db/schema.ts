import { integer, real, sqliteTable, text } from "drizzle-orm/sqlite-core";

export const projects = sqliteTable("projects", {
  id: text("id").primaryKey(),
  userId: text("user_id").notNull(),
  name: text("name").notNull(),
  createdAt: integer("created_at", { mode: "timestamp" }).notNull(),
});

export const analysisCases = sqliteTable("analysis_cases", {
  id: text("id").primaryKey(),
  projectId: text("project_id").notNull(),
  name: text("name").notNull(),
  toolType: text("tool_type", {
    enum: ["response_spectrum", "time_history", "section"],
  }).notNull(),
  comment: text("comment"),
  createdAt: integer("created_at", { mode: "timestamp" }).notNull(),
});

export const analysisSets = sqliteTable("analysis_sets", {
  id: text("id").primaryKey(),
  analysisCaseId: text("analysis_case_id").notNull(),
  waveformId: text("waveform_id"),
  damping: real("damping"),
  status: text("status", { enum: ["pending", "done", "error"] })
    .notNull()
    .default("pending"),
  r2Key: text("r2_key"),
  calculatedAt: integer("calculated_at", { mode: "timestamp" }),
  createdAt: integer("created_at", { mode: "timestamp" }).notNull(),
});

export const waveforms = sqliteTable("waveforms", {
  id: text("id").primaryKey(),
  userId: text("user_id").notNull(),
  name: text("name").notNull(),
  kind: text("kind", { enum: ["observed", "code"] }).notNull(),
  dt: real("dt").notNull(),
  duration: real("duration").notNull(),
  pgv: real("pgv"),
  r2Key: text("r2_key").notNull(),
  createdAt: integer("created_at", { mode: "timestamp" }).notNull(),
});
