import { drizzle } from "drizzle-orm/d1";
import * as authSchema from "./auth.schema";
import * as appSchema from "./schema";

export const schema = { ...authSchema, ...appSchema };

export const createDb = (d1: D1Database) => drizzle(d1, { schema });
