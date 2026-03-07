import { JuiceElement, type JuiceElementProps } from "./JuiceElement.js";

export default class JuiceLayoutElement<
  Props extends JuiceElementProps = JuiceElementProps,
> extends JuiceElement<Props> {
  public readonly nodeId: number;

  constructor(tagName: string) {
    super(tagName);
    this.nodeId = dom.createElement(tagName);
  }

  setAttribute(key: string, value: unknown): void {
    if (key === "width" || key === "height") {
      this.style[key] = value;
    } else {
      super.setAttribute(key, value);
    }
  }
}
