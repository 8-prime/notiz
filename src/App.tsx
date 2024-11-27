import Titlebar from "./components/Titlebar";
import NoteEditor from "./components/NoteEditor";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import NoteOverview from "./components/NoteOverview";

function App() {
  return (
    <main className="flex flex-col items-center justify-start h-screen w-screen">
      <Titlebar />
      <BrowserRouter>
        <Routes>
          <Route path="/" element={<NoteEditor />} />
          <Route path="/search" element={<NoteOverview />} />
        </Routes>
      </BrowserRouter>
    </main>
  );
}

export default App;
