import { cmd } from "@/lib/std/cmd.ts";

async function run(args: string[]): Promise<void> {
  await cmd.run("runseal", args);
}

async function text(args: string[]): Promise<string> {
  return await cmd.text("runseal", args);
}

export const runseal = {
  run,
  text,
};
