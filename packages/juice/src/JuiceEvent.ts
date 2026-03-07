import type { JuiceNode } from "./JuiceNode.js";

export class JuiceEvent<
  T extends Record<string, unknown> = Record<never, unknown>,
> {
  public readonly type: string;
  public readonly target: JuiceNode;
  public readonly details: T;
  private _propagationStopped: boolean;

  constructor(type: string, target: JuiceNode, details: T) {
    this.type = type;
    this.target = target;
    this.details = details;
    this._propagationStopped = false;
  }

  stopPropagation() {
    this._propagationStopped = true;
  }

  get propagationStopped(): boolean {
    return this._propagationStopped;
  }
}

export class PressEvent extends JuiceEvent<{ x: number; y: number }> {}

export interface UIEventMap {
  PressIn: PressEvent;
  PressOut: PressEvent;
  Press: PressEvent;
  PressMove: PressEvent;
}

export type UIEventListener<Event extends keyof UIEventMap> = (
  event: UIEventMap[Event],
) => void;
