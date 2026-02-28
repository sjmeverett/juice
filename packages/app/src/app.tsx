import { Box } from "@juice/core";
import { useEffect, useState } from "preact/hooks";
import Button from "./Button.js";

export function App() {
	const [count, setCount] = useState(0);

	useEffect(() => {
		console.log("useEffect called");
		const handle = setInterval(() => {
			setCount((count) => count + 1);
		}, 1000);

		return () => clearInterval(handle);
	}, []);

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
				Hello, World
			</Box>
			<Box>Count: {count}</Box>
			<Button
				onPress={() => {
					setCount(count + 1);
					console.log(`Counter incremented to ${count + 1}`);
				}}
				buttonColor={["#ff8000", "#ff4000"]}
				style={{
					padding: 20,
					fontSize: 36,
					color: "#000000",
					borderRadius: 5,
					marginTop: 50,
					alignSelf: "flex-start",
				}}
			>
				Increment
			</Button>
		</Box>
	);
}
