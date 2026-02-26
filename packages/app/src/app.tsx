declare function nativeLog(message: string): void;

import { Box } from "@jsui/core";
import { useState } from "preact/hooks";

export function App() {
	const [count, setCount] = useState(0);
	return (
		<Box
			style={{
				background: "#000000",
				flexDirection: "column",
				width: "100%",
				height: "100%",
				padding: 20,
				gap: 12,
			}}
		>
			<Box style={{ color: "#ffffff", font: "Roboto-Bold", fontSize: 72 }}>
				Hello, World!
			</Box>
			<Box>Count: {count}</Box>
			<Box
				onPress={() => {
					setCount(count + 1);
					nativeLog(`Counter incremented to ${count + 1}`);
				}}
				style={{
					padding: 20,
					fontSize: 36,
					background: "#ff8000",
					color: "#000000",
					borderRadius: 5,
				}}
			>
				Increment
			</Box>
		</Box>
	);
}
