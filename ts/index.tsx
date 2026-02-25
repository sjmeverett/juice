import "./fake-dom";
import { render } from "preact";
import { App } from "./app";
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

const container = document.createElement("root") as unknown as NativeNode;
render(<App />, container as any);

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

globalThis.__refreshTree__ = () => {
	nodeRegistry = new Map();
	nextNodeId = 0;
	globalThis.__TREE__ = JSON.stringify(serialize(container), null, 2);
};

globalThis.__dispatchEvent__ = (nodeId: number, eventType: string) => {
	const node = nodeRegistry.get(nodeId);
	if (node) {
		node.dispatchEvent({ type: eventType } as Event);
	}
};

globalThis.__refreshTree__();
