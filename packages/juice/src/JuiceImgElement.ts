import type { JuiceElementProps } from "./JuiceElement.js";
import JuiceLayoutElement from "./JuiceLayoutElement.js";

interface JuiceImgElementProps extends JuiceElementProps {
  src?: string;
  width?: number | string;
  height?: number | string;
}

export class JuiceImgElement extends JuiceLayoutElement<JuiceImgElementProps> {
  constructor() {
    super("img");
  }
}
