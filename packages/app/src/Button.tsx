import { Box, type BoxProps } from "@juice/core";
import { useState } from "preact/hooks";

export interface ButtonProps extends BoxProps {
	buttonColor: [string, string];
}

export default function Button({
	style,
	buttonColor,
	onPressIn,
	onPressOut,
	...props
}: ButtonProps) {
	const [pressed, setPressed] = useState(false);

	return (
		<Box
			style={{
				...style,
				background: pressed ? buttonColor[1] : buttonColor[0],
			}}
			onPressIn={(e) => {
				setPressed(true);
				onPressIn?.(e);
			}}
			onPressOut={(e) => {
				setPressed(false);
				onPressOut?.(e);
			}}
			{...props}
		/>
	);
}
