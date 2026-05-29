// Minimal WebSocket echo server for testing Syncara WS proxying.
// Usage: node server.js

import { createServer } from "http";
import { WebSocketServer } from "ws";

const server = createServer();
const wss = new WebSocketServer({ server });

wss.on("connection", (ws, req) => {
  console.log(`WS client connected — ${req.socket.remoteAddress}`);

  ws.on("message", (data) => {
    console.log(`echo: ${data}`);
    ws.send(`echo: ${data}`);
  });

  ws.on("close", () => console.log("WS client disconnected"));
  ws.send("connected to syncara backend");
});

server.listen(9003, () => console.log("WS backend on :9003"));
