import type { UIEvent, UIEventListener, UIEventMap } from "./UIEvent.js";

export abstract class UINode {
  public readonly nodeType: number;
  public readonly namespaceURI: string | null;
  public readonly childNodes: UINode[];

  public id?: number;
  public parentNode: UINode | null;

  private eventListeners: Map<string, Set<Function>> = new Map();

  public static ELEMENT_NODE = 1;
  public static TEXT_NODE = 3;

  constructor(nodeType: number, namespaceURI: string | null = null) {
    this.nodeType = nodeType;
    this.namespaceURI = namespaceURI;
    this.parentNode = null;
    this.childNodes = [];
  }

  get firstChild() {
    return this.childNodes[0] ?? null;
  }

  get nextSibling(): UINode | null {
    if (!this.parentNode) return null;

    const siblings = this.parentNode.childNodes;
    return siblings[siblings.indexOf(this) + 1] ?? null;
  }

  contains(other: UINode | null): boolean {
    if (other === null) return false;
    if (other === this) return true;

    return this.childNodes.some(
      (child) => child === other || child.contains(other),
    );
  }

  insertBefore(node: UINode, child: UINode | null): UINode {
    node.parentNode = this;
    const idx = child ? this.childNodes.indexOf(child) : -1;

    if (idx >= 0) {
      this.childNodes.splice(idx, 0, node);
    } else {
      this.childNodes.push(node);
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

  dispatchEvent(event: UIEvent) {
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

  appendChild(node: UINode): UINode {
    node.parentNode = this;
    this.childNodes.push(node);
    return node;
  }

  removeChild(child: UINode): UINode {
    const idx = this.childNodes.indexOf(child);

    if (idx >= 0) {
      this.childNodes.splice(idx, 1);
    }

    child.parentNode = null;
    return child;
  }

  prepend(node: UINode): UINode {
    node.parentNode = this;
    this.childNodes.unshift(node);
    return node;
  }

  abstract toJSON(): unknown;
}
