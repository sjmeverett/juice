declare function nativeLog(message: string): void;

import { Button, Label, Screen } from "@jsui/core";
import { useState } from "preact/hooks";

export function App() {
	const [count, setCount] = useState(0);
	return (
		<Screen style={{ background: "#000000" }}>
			<Label style={{ color: "#ffffff", font: "Roboto-Bold", fontSize: 72 }}>
				Hello, World!
			</Label>
			<Label>Count: {count}</Label>
			<Button
				onPress={() => {
					setCount(count + 1);
					nativeLog(`Counter incremented to ${count + 1}`);
				}}
			>
				Increment
			</Button>
		</Screen>
	);
}
