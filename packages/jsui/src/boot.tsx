import "./fake-dom";
import { type ComponentChild, render } from "preact";
import { type NativeNode, NativeTextNode } from "./fake-dom";

type FakeNode = NativeNode | NativeTextNode;

interface SerializedText {
	type: "#text";
	text: string;
}

interface SerializedNode {
	type: string;
	props: Record<string, unknown>;
	children: SerializedTree[];
}

type SerializedTree = SerializedText | SerializedNode;

export function boot(app: ComponentChild) {
	const container = document.createElement("root") as unknown as NativeNode;
	render(app, container as any);

	let nodeRegistry = new Map<number, NativeNode>();
	let nextNodeId = 0;

	function serialize(node: FakeNode): SerializedTree {
		if (node instanceof NativeTextNode) {
			return { type: "#text", text: node.data };
		}
		const id = nextNodeId++;
		node._id = id;
		nodeRegistry.set(id, node);

		const props: Record<string, unknown> = { ...node.props };
		if (node.style && Object.keys(node.style).length > 0) {
			props.style = node.style;
		}
		props.id = id;
		return {
			type: node.type,
			props,
			children: node.children.map(serialize),
		};
	}

	function flushTree() {
		nodeRegistry = new Map();
		nextNodeId = 0;
		renderer.setTree(JSON.stringify(serialize(container)));
	}

	(document as any).onEvent = (nodeId: number, eventType: string) => {
		const node = nodeRegistry.get(nodeId);
		if (node) {
			node.dispatchEvent({ type: eventType } as Event);
		}
		flushTree();
	};

	flushTree();
}
