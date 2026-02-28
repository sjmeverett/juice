#!/usr/bin/env tsx

import { startDevServer } from "./dev-server.js";

const args = process.argv.slice(2);
const command = args[0];

if (command === "dev") {
	let port = 3000;
	let entryPoint: string | undefined;

	for (let i = 1; i < args.length; i++) {
		if (args[i] === "--port" || args[i] === "-p") {
			port = Number.parseInt(args[++i], 10);
			if (Number.isNaN(port)) {
				console.error("Error: --port requires a number");
				process.exit(1);
			}
		} else if (!args[i].startsWith("-")) {
			entryPoint = args[i];
		}
	}

	if (!entryPoint) {
		console.error("Usage: juice dev <entrypoint> [--port <port>]");
		process.exit(1);
	}

	startDevServer({ entryPoint, port });
} else {
	console.error("Usage: juice <command>");
	console.error("");
	console.error("Commands:");
	console.error("  dev <entrypoint>   Start the dev server with hot reloading");
	process.exit(1);
}
