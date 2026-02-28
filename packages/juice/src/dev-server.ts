import { createServer } from "node:http";
import { context } from "esbuild";
import { type WebSocket, WebSocketServer } from "ws";

interface DevServerOptions {
  entryPoint: string;
  port: number;
}

export async function startDevServer({ entryPoint, port }: DevServerOptions) {
  let currentBundle = "";
  const clients = new Set<WebSocket>();

  const ctx = await context({
    entryPoints: [entryPoint],
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
    plugins: [
      {
        name: "hot-reload",
        setup(build) {
          build.onEnd((result) => {
            if (result.errors.length > 0 || !result.outputFiles) return;

            currentBundle = result.outputFiles[0].text;

            console.log(
              `[dev] rebuilt (${(currentBundle.length / 1024).toFixed(1)}kb), notifying ${clients.size} client(s)`,
            );

            for (const client of clients) {
              if (client.readyState === 1) {
                client.send(currentBundle);
              }
            }
          });
        },
      },
    ],
  });

  await ctx.watch();

  const server = createServer((req, res) => {
    if (req.url === "/bundle.js") {
      res.writeHead(200, { "Content-Type": "application/javascript" });
      res.end(currentBundle);
    } else {
      res.writeHead(404);
      res.end();
    }
  });

  const wss = new WebSocketServer({ noServer: true });

  server.on("upgrade", (req, socket, head) => {
    wss.handleUpgrade(req, socket, head, (ws) => wss.emit("connection", ws));
  });

  wss.on("connection", (ws) => {
    clients.add(ws);
    console.log(`[dev] client connected (${clients.size} total)`);
    if (currentBundle) ws.send(currentBundle);
    ws.on("close", () => {
      clients.delete(ws);
      console.log(`[dev] client disconnected (${clients.size} total)`);
    });
  });

  server.listen(port, () => {
    console.log(`[dev] listening on http://localhost:${port}`);
    console.log(
      `[dev] set DEV_SERVER=ws://localhost:${port} on your Rust binary`,
    );
  });
}
