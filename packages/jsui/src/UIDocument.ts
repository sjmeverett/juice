import { UIElement } from "./UIElement.js";
import { PressEvent } from "./UIEvent.js";
import type { UINode } from "./UINode.js";
import { UITextNode } from "./UITextNode.js";

export class UIDocument extends UIElement {
	constructor() {
		super("document");

		let pressedNode: UINode | undefined;

		this.addEventListener("PressIn", (event) => {
			pressedNode = event.target;
		});

		this.addEventListener("PressOut", (event) => {
			if (pressedNode?.contains(event.target)) {
				pressedNode.dispatchEvent(
					new PressEvent(
						"Press",
						pressedNode,
						event.details as { x: number; y: number },
					),
				);
			}

			pressedNode = undefined;
		});
	}

	createElement(tagName: string): UIElement {
		return new UIElement(tagName);
	}

	createElementNS(namespaceURI: string, tagName: string): UIElement {
		return new UIElement(tagName, namespaceURI);
	}

	createTextNode(text: string): UITextNode {
		return new UITextNode(text);
	}

	get documentElement(): UIElement {
		return this;
	}
}

export const document = new UIDocument();
(globalThis as unknown as { document: UIDocument }).document = document;
