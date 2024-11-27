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


    return (
        <div className="w-full h-full flex flex-col items-center justify-start">
            <h1 className="text-3xl font-bold">Notiz</h1>
            <div>
                {notes.map((note) => {
                    return (
                        <div key={note.id} className="w-full h-full flex flex-col items-center justify-start">
                            <a href={`/${note.id}`}>
                                <h1 className="text-xl font-bold">{note.content.substring(0, 20)}</h1>
                            </a>
                        </div>
                    );
                })}
            </div>
        </div>
    );
}