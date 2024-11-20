import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import Titlebar from "./components/titlebar";
import NoteEditor from "./components/NoteEditor";

function App() {
  const [greetMsg, setGreetMsg] = useState("");
  const [name, setName] = useState("");

  async function greet() {
    // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
    setGreetMsg(await invoke("greet", { name }));
  }

  return (
    <main className="flex flex-col items-center justify-start h-screen w-screen">
      <Titlebar />
      <NoteEditor />
    </main>
  );
}

export default App;
