import { betterAuth } from "better-auth";
import { drizzleAdapter } from "better-auth/adapters/drizzle";
import { admin } from "better-auth/plugins";
import { Resend } from "resend";
import { createDb, schema } from "../db";

export const createAuth = (env: CloudflareBindings) => {
  const db = createDb(env.DB);
  const resend = new Resend(env.RESEND_API_KEY);

  return betterAuth({
    baseURL: env.BETTER_AUTH_URL,
    secret: env.BETTER_AUTH_SECRET,
    trustedOrigins: [env.BETTER_AUTH_URL, "http://localhost:8787", "http://localhost:5173"],
    database: drizzleAdapter(db, {
      provider: "sqlite",
      schema,
    }),
    emailAndPassword: {
      enabled: true,
      requireEmailVerification: true,
      customSyntheticUser: ({ coreFields, additionalFields, id }) => ({
        ...coreFields,
        role: "user",
        banned: false,
        banReason: null,
        banExpires: null,
        ...additionalFields,
        id,
      }),
    },
    emailVerification: {
      sendVerificationEmail: async ({ user, url }) => {
        const { data, error } = await resend.emails.send({
          from: "Rustruct <noreply@matsumok.com>",
          to: user.email,
          subject: "メールアドレスの確認",
          html: `<p>以下のリンクをクリックしてメールアドレスを確認してください：</p><p><a href="${url}">${url}</a></p>`,
        });
        if (error) console.error("[Resend]", error);
        else console.log("[Resend] sent", data?.id);
      },
    },
    plugins: [admin()],
  });
};
