import type { ComponentChildren } from "preact";

// -- Style & prop types --

export interface BoxStyle {
	// Visual
	background?: string;
	color?: string;
	font?: string;
	fontSize?: number;
	// Layout
	flexDirection?: "row" | "column";
	flexGrow?: number;
	flexShrink?: number;
	width?: string | number;
	height?: string | number;
	padding?: number;
	paddingLeft?: number;
	paddingRight?: number;
	paddingTop?: number;
	paddingBottom?: number;
	gap?: number;
}

interface BaseProps {
	children?: ComponentChildren;
	style?: BoxStyle;
}

interface ButtonProps extends BaseProps {
	onPress?: () => void;
	onPressIn?: () => void;
	onPressOut?: () => void;
}

// -- Intrinsic element: single generic "box" --

declare module "preact" {
	namespace JSX {
		interface IntrinsicElements {
			box: BaseProps & {
				onPressIn?: () => void;
				onPressOut?: () => void;
			};
		}
	}
}

// -- Wrapper components with default styles --

export function Screen(props: BaseProps) {
	return (
		<box
			{...props}
			style={{
				flexDirection: "column",
				width: "100%",
				height: "100%",
				padding: 20,
				gap: 12,
				...props.style,
			}}
		/>
	);
}

export function Label(props: BaseProps) {
	return (
		<box
			{...props}
			style={{
				flexDirection: "row",
				...props.style,
			}}
		/>
	);
}

export function Button(props: ButtonProps) {
	const { onPress, onPressIn, onPressOut, ...rest } = props;
	return (
		<box
			{...rest}
			onPressIn={onPressIn}
			onPressOut={() => {
				onPressOut?.();
				onPress?.();
			}}
			style={{
				flexDirection: "row",
				padding: 8,
				...props.style,
			}}
		/>
	);
}
