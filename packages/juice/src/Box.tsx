import type { JuiceElementProps } from "./JuiceElement.js";

export interface BoxProps extends JuiceElementProps {}

declare module "preact" {
  namespace JSX {
    interface IntrinsicElements {
      box: BoxProps;
    }
  }
}

export function Box(props: BoxProps) {
  return <box {...props} />;
}
