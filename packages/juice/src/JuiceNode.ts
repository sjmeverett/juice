import type { JuiceEvent, UIEventListener, UIEventMap } from "./JuiceEvent.js";

export class JuiceNode {
  public readonly nodeType: number;
  public readonly namespaceURI: string | undefined;
  public readonly childNodes: JuiceNode[];

  public parentNode: JuiceNode | null;
  public nodeId: number | undefined = undefined;

  private eventListeners: Map<string, Set<Function>> = new Map();

  public static ELEMENT_NODE = 1;
  public static TEXT_NODE = 3;

  constructor(
    nodeType: number,
    namespaceURI: string | undefined = undefined,
    nodeId: number | undefined = undefined,
  ) {
    this.nodeType = nodeType;
    this.namespaceURI = namespaceURI;
    this.nodeId = nodeId;
    this.parentNode = null;
    this.childNodes = [];
  }

  get firstChild() {
    return this.childNodes[0] ?? null;
  }

  get nextSibling(): JuiceNode | null {
    if (!this.parentNode) {
      return null;
    }

    const siblings = this.parentNode.childNodes;
    return siblings[siblings.indexOf(this) + 1] ?? null;
  }

  contains(other: JuiceNode | null): boolean {
    if (other === null) return false;
    if (other === this) return true;

    return this.childNodes.some(
      (child) => child === other || child.contains(other),
    );
  }

  insertBefore(node: JuiceNode, child: JuiceNode | null): JuiceNode {
    node.parentNode = this;
    const idx = child ? this.childNodes.indexOf(child) : -1;

    if (child && idx >= 0) {
      this.childNodes.splice(idx, 0, node);

      if (this.nodeId && node.nodeId) {
        dom.insertChildAt(idx, this.nodeId, node.nodeId);
      }
    } else {
      this.appendChild(node);
    }

    return node;
  }

  private getEventListeners(type: string) {
    const listeners = this.eventListeners.get(type);

    if (listeners) {
      return listeners;
    }

    const newListeners = new Set<Function>();
    this.eventListeners.set(type, newListeners);

    return newListeners;
  }

  addEventListener<E extends keyof UIEventMap>(
    type: E,
    listener: UIEventListener<E>,
  ): () => void {
    const listeners = this.getEventListeners(type);
    listeners.add(listener);

    return () => listeners.delete(listener);
  }

  removeEventListener<E extends keyof UIEventMap>(
    type: E,
    listener: UIEventListener<E>,
  ): void {
    this.eventListeners.get(type)?.delete(listener);
  }

  dispatchEvent(event: JuiceEvent) {
    const listeners = this.eventListeners.get(event.type);

    if (listeners) {
      for (const fn of listeners) {
        fn.call(this, event);
      }
    }

    if (!event.propagationStopped) {
      this.parentNode?.dispatchEvent(event);
    }
  }

  appendChild(node: JuiceNode): JuiceNode {
    node.parentNode = this;
    this.childNodes.push(node);

    if (this.nodeId && node.nodeId) {
      dom.appendChild(this.nodeId, node.nodeId);
    }

    return node;
  }

  removeChild(child: JuiceNode): JuiceNode {
    const idx = this.childNodes.indexOf(child);

    if (idx >= 0) {
      this.childNodes.splice(idx, 1);
    }

    child.parentNode = null;

    if (this.nodeId && child.nodeId) {
      dom.removeChild(this.nodeId, child.nodeId);
    }

    return child;
  }

  prepend(node: JuiceNode): JuiceNode {
    node.parentNode = this;
    this.childNodes.unshift(node);

    if (this.nodeId && node.nodeId) {
      dom.insertChildAt(0, this.nodeId, node.nodeId);
    }

    return node;
  }
}
