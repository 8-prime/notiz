import { invoke } from "@tauri-apps/api/core";
import React from "react";
import { useEffect } from "react";
import { Note } from "../models/Note";

export default function NoteOverview() {

    const [notes, setNotes] = React.useState<Note[]>([]);

    useEffect(() => {
        invoke("get_notes").then((notes) => {
            setNotes(notes as Note[]);
        });
    }, []);


    const openArticle = (id: string | undefined) => {
        if (!id) return;
        invoke("open_article", { id: id })
    }

    return (
        <div className="w-full h-full flex flex-col items-center justify-start">
            <h1 className="text-3xl font-bold">Notiz</h1>
            <div>
                {notes.map((note) => {
                    return (
                        <div key={note.id} className="w-full h-full flex flex-col items-center justify-start">
                            <button onClick={() => openArticle(note.id)} className="w-full h-full flex flex-col items-center justify-start">
                                <h1 className="text-xl font-bold">{note.content.substring(0, 20)}</h1>
                            </button>
                        </div>
                    );
                })}
            </div>
        </div>
    );
}