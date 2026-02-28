import circleImageIcon from "@iconify/icons-fluent/circle-image-24-regular.js";
import { Box } from "@juice/core";
import { useState } from "preact/hooks";
import Button from "./Button.js";
import Icon from "./Icon.js";

export function App() {
  const [count, setCount] = useState(0);

  return (
    <Box
      style={{
        background: "#000000",
        flexDirection: "column",
        width: "100%",
        height: "100%",
        padding: 20,
        gap: 12,
      }}
    >
      <Box style={{ color: "#ffffff", font: "Roboto-Bold", fontSize: 72 }}>
        Hello, World
      </Box>
      <Box>Count: {count}</Box>
      <Button
        onPress={() => {
          setCount(count + 1);
          console.log(`Counter incremented to ${count + 1}`);
        }}
        buttonColor={["#ff8000", "#ff4000"]}
        style={{
          padding: 20,
          fontSize: 36,
          color: "#000000",
          borderRadius: 5,
          marginTop: 50,
          alignSelf: "flex-start",
        }}
      >
        <Icon icon={circleImageIcon} size={48} />
        Increment
      </Button>
    </Box>
  );
}