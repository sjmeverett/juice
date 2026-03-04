const esbuild = require("esbuild");

esbuild
  .build({
    entryPoints: ["packages/app/src/index.tsx"],
    bundle: true,
    format: "iife",
    write: false,
    jsx: "automatic",
    jsxImportSource: "preact",
    loader: {
      ".png": "dataurl",
      ".jpg": "dataurl",
      ".jpeg": "dataurl",
      ".gif": "dataurl",
      ".webp": "dataurl",
      ".ttf": "dataurl",
    },
    outfile: "dist/bundle.js",
  })
  .then(() => {
    console.log("Build succeeded!");
  })
  .catch((e) => {
    console.error(e);
    process.exit(1);
  });
