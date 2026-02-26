import {
	type ComponentChild,
	type ContainerNode,
	options,
	render,
} from "preact";
import { PressEvent, UIElement, type UINode, UITextNode } from "./dom.js";

interface SerializedText {
	type: "#text";
	text: string;
}

interface SerializedElement {
	type: string;
	props: Record<string, unknown>;
	children: SerializedNode[];
}

type SerializedNode = SerializedText | SerializedElement;

export function createRoot(): Root {
	// make preact render synchronously
	options.debounceRendering = (cb) => cb();
	return new Root();
}

export class Root {
	private readonly container: UIElement;

	constructor() {
		this.container = new UIElement("root");
		this.wireUpDerivedEvents(this.container);
	}

	render(app: ComponentChild) {
		render(app, this.container as unknown as ContainerNode);
		this.update();
	}

	serialize(node: UINode, allNodes: UINode[]): SerializedNode {
		if (node instanceof UITextNode) {
			return { type: "#text", text: node.textContent };
		} else if (node instanceof UIElement) {
			node.id = allNodes.length;
			allNodes.push(node);

			const props: Record<string, unknown> = { ...node.props, id: node.id };
			if (Object.keys(node.style).length > 0) {
				props.style = node.style;
			}

			return {
				type: node.tagName,
				props,
				children: node.childNodes.map((child) =>
					this.serialize(child, allNodes),
				),
			};
		} else {
			throw new Error(`Unsupported node type: ${node.constructor.name}`);
		}
	}

	update() {
		const nodes: UINode[] = [];
		const contents = JSON.stringify(this.serialize(this.container, nodes));

		renderer.update(contents, (nodeId, event) => {
			nativeLog(`Node ${nodeId} received event ${event.type}`);
			const node = nodes[nodeId];

			if (node) {
				node.dispatchEvent(
					new PressEvent(
						event.type,
						node,
						event.details as { x: number; y: number },
					),
				);
			}

			this.update();
		});
	}

	private wireUpDerivedEvents(root: UIElement) {
		let pressedNode: UINode | undefined;

		root.addEventListener("PressIn", (event) => {
			pressedNode = event.target;
		});

		root.addEventListener("PressOut", (event) => {
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
}
