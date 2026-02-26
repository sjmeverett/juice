import type { UIElementProps } from "./dom.js";

export interface BoxStyle {
	background?: string;
	color?: string;
	flexDirection?: "row" | "column";
	flexGrow?: number;
	flexShrink?: number;
	font?: string;
	fontSize?: number;
	gap?: number;
	height?: string | number;
	padding?: number;
	paddingBottom?: number;
	paddingLeft?: number;
	paddingRight?: number;
	paddingTop?: number;
	width?: string | number;
}

export interface BoxProps extends UIElementProps {
	style?: BoxStyle;
}

declare module "preact" {
	namespace JSX {
		interface IntrinsicElements {
			box: BoxProps;
		}
	}
}

export function Box(props: BoxProps) {
	return <box {...props} />;
}
