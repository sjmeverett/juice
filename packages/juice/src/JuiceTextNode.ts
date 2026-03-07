import { JuiceNode } from "./JuiceNode.js";

export class JuiceTextNode extends JuiceNode {
  public readonly nodeId: number;
  private text = "";

  constructor(text: string) {
    super(JuiceNode.TEXT_NODE);
    this.text = String(text);
    this.nodeId = dom.createTextNode(this.text);
  }

  get data(): string {
    return this.text;
  }

  set data(text: string) {
    this.text = String(text);
    dom.setAttributeString(this.nodeId, "text", this.text);
  }

  toJSON() {
    // make it easy to dump the DOM for debugging
    return { id: this.nodeId, tag: "#text", text: this.text };
  }
}
