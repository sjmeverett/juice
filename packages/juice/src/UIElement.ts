import type { ComponentChildren } from "preact";
import type { UIEventListener, UIEventMap } from "./UIEvent.js";
import { UINode } from "./UINode.js";
import {
	acceptNumber,
	acceptPixelsOrPercent,
	acceptString,
	calculateTRBL,
	parseColor,
	parsePixels,
} from "./util.js";

export type UIElementProps = {
	[K in keyof UIEventMap as `on${Capitalize<K>}`]?: UIEventListener<K>;
} & {
	children?: ComponentChildren;
};

let nextNodeId = 0;

export class UIElement<
	Props extends UIElementProps = UIElementProps,
> extends UINode {
	public readonly nodeId;
	public readonly tagName: string;
	public readonly props: Partial<Props> = {};
	public style: Record<string, unknown> = {};

	constructor(tagName: string, namespaceURI: string | undefined = undefined) {
		super(UINode.ELEMENT_NODE, namespaceURI);
		this.tagName = tagName;
		this.nodeId = nextNodeId++;
	}

	setAttribute(key: string, val: unknown): void {
		(this.props as Record<string, unknown>)[key] = val;
	}

	removeAttribute(key: string): void {
		delete (this.props as Record<string, unknown>)[key];
	}

	toJSON() {
		const style = this.style;

		return {
			type: "element",
			tag:
				this.namespaceURI &&
				this.namespaceURI !== "http://www.w3.org/1999/xhtml"
					? `${this.namespaceURI}:${this.tagName}`
					: this.tagName,
			id: this.nodeId,
			children: this.childNodes,

			alignItems: acceptString(style.alignItems),
			alignSelf: acceptString(style.alignSelf),
			background: parseColor(style.background),
			borderRadius: parsePixels(style.borderRadius),
			color: parseColor(style.color),
			flexBasis: parsePixels(style.flexBasis),
			flexDirection: acceptString(style.flexDirection),
			flexGrow: acceptNumber(style.flexGrow),
			flexShrink: acceptNumber(style.flexShrink),
			font: acceptString(style.font),
			fontSize: parsePixels(style.fontSize),
			gap: parsePixels(style.gap),
			height: acceptPixelsOrPercent(style.height),
			width: acceptPixelsOrPercent(style.width),

			margin: calculateTRBL(
				style.margin,
				style.marginX,
				style.marginY,
				style.marginTop,
				style.marginRight,
				style.marginBottom,
				style.marginLeft,
			),

			padding: calculateTRBL(
				style.padding,
				style.paddingX,
				style.paddingY,
				style.paddingTop,
				style.paddingRight,
				style.paddingBottom,
				style.paddingLeft,
			),
		};
	}

	findElementByNodeId(id: number): UIElement | undefined {
		if (this.nodeId === id) {
			return this;
		}

		for (const child of this.childNodes) {
			if (!(child instanceof UIElement)) {
				continue;
			}

			const found = child.findElementByNodeId(id);

			if (found) {
				return found;
			}
		}

		return undefined;
	}
}
