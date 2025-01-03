import Titlebar from "./components/Titlebar";
import NoteEditor from "./components/NoteEditor";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import NoteOverview from "./components/NoteOverview";
import { useEffect } from "react";
import setupAppWindow from "./utils/utils";

function App() {

  useEffect(() => {
    setupAppWindow();
  }, [])

  return (
    <main className="flex flex-col items-center justify-start h-screen w-screen  bg-neutral-950">
      <Titlebar />
      <BrowserRouter>
        <Routes>
          <Route path="/:id" element={<NoteEditor />} />
          <Route path="/" element={<NoteEditor />} />
          <Route path="/search" element={<NoteOverview />} />
        </Routes>
      </BrowserRouter>
    </main>
  );
}

export default App;
