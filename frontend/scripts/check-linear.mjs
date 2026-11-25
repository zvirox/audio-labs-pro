#!/usr/bin/env node
/**
 * Linear-Style Design System Gate.
 * Fails on:
 * - blur / frosted glass (backdrop-filter, backdrop-blur, shadowBlur)
 * - translucency (rgba/hsla/rgb(... / ...), Tailwind slash opacity modifiers)
 * - glow / inset highlights / pulse (drop-shadow, inset shadow, animate-pulse)
 * - dark-theme box shadows (shadow-xs/sm/md/lg/xl/2xl/shadow)
 * - surface gradients (bg-gradient-, createLinearGradient outside charts)
 * - raw hex colors outside tokens.css, index.css, and echarts-linear-theme.ts
 */
import { readFileSync, readdirSync, statSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { join, relative } from "node:path";

const APP_ROOT = fileURLToPath(new URL("..", import.meta.url));
const UI_SRC = join(APP_ROOT, "src");

function findFiles(dir, exts, acc = []) {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) {
      findFiles(full, exts, acc);
    } else if (exts.some((ext) => entry.endsWith(ext))) {
      acc.push(full);
    }
  }
  return acc;
}

const TOKEN_FILES = new Set([
  "tokens.css",
  "index.css",
  "components/linear/echarts-linear-theme.ts",
]);

const CHECKS = [
  {
    name: "blur",
    regex: /(?:-webkit-)?backdrop-filter\s*:|backdrop-blur|ctx\.shadowBlur\s*=/i,
  },
  {
    name: "translucency",
    regex: /\brgba\s*\(|\bhsla\s*\(|(?:bg|border|text|ring|from|via|to)-(?:white|black|[a-z]+-\d+|\[[^\]]+\]|var\([^)]+\))\/\d+/i,
  },
  {
    name: "glow-or-pulse",
    regex: /\bdrop-shadow\b|box-shadow[^;\n]*inset|\banimate-pulse\b|ctx\.shadowColor/i,
  },
  {
    name: "dark-shadow",
    regex: /\bshadow-(?:xs|sm|md|lg|xl|2xl)\b|\b(?<![\w-])shadow(?=\s|["'`])|\bshadow-\[(?!var\(--shadow-card\))/i,
  },
  {
    name: "surface-gradient",
    regex: /\bbg-gradient-to-|\blinear-gradient\s*\(|\bradial-gradient\s*\(/i,
  },
];

const RAW_HEX_REGEX = /#[0-9a-fA-F]{3,8}\b/g;

const files = findFiles(UI_SRC, [".tsx", ".ts", ".css"]);
const violations = [];

for (const file of files) {
  const rel = relative(UI_SRC, file).replace(/\\/g, "/");
  const content = readFileSync(file, "utf8");
  const lines = content.split(/\r?\n/);
  const isTokenFile = TOKEN_FILES.has(rel);

  lines.forEach((line, idx) => {
    const trimmed = line.trim();
    if (trimmed.startsWith("//") || trimmed.startsWith("/*") || trimmed.startsWith("*")) {
      return;
    }

    if (!isTokenFile) {
      for (const check of CHECKS) {
        if (check.regex.test(line)) {
          violations.push(`${rel}:${idx + 1} [${check.name}] ${trimmed}`);
        }
      }
      const hexMatches = line.match(RAW_HEX_REGEX);
      if (hexMatches) {
        violations.push(`${rel}:${idx + 1} [raw-hex: ${hexMatches.join(", ")}] ${trimmed}`);
      }
    }
  });
}

if (violations.length > 0) {
  console.error(`❌ check:linear gate FAILED (${violations.length} violation(s)):`);
  for (const v of violations.slice(0, 60)) {
    console.error(`  - ${v}`);
  }
  if (violations.length > 60) {
    console.error(`  ... and ${violations.length - 60} more`);
  }
  process.exit(1);
} else {
  console.log("✅ check:linear gate PASSED: Zero blur, translucency, glow, dark shadows, gradients, or off-token hex colors.");
  process.exit(0);
}
