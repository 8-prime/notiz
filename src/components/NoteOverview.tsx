import { invoke } from "@tauri-apps/api/core";
import { useEffect, useState } from "react";
import { Note } from "../models/Note";
import { Star, Trash2 } from "lucide-react";
import { useDebouncedCallback } from "use-debounce";

function ArticleTitle({ note }: Readonly<{ note: Note }>) {
    const title = note.content.substring(0, 20);
    return (
        <>
            {title.length === 0 &&
                <p className="italic text-lg">No title</p>
            }
            {
                title.length > 0 &&
                <p className="text-lg font-medium">{title}</p>
            }
        </>
    )
}

function ArticleInfo({ note, selected }: Readonly<{ note: Note, selected: string | undefined }>) {
    const [showDelete, setShowDelete] = useState<boolean>(false);
    const openArticle = (id: string | undefined) => {
        if (!id) return;
        invoke("open_article", { id: id })
    }

    return (
        <div className={`grid grid-rows-1 grid-cols-1 h-12  hover:bg-neutral-700 rounded-lg p-2 ${selected === note.id ? 'bg-neutral-800' : ''}`}>
            <div className="col-start-1 row-start-1 flex flex-row items-center justify-start gap-2">
                <button onClick={() => openArticle(note.id)} className="grow h-full flex items-center justify-start">
                    <ArticleTitle note={note}></ArticleTitle>
                </button>
                {
                    !showDelete &&
                    <>
                        <button>
                            <Star color="#7a7a7a" />
                        </button>
                        <button onClick={() => setShowDelete(true)}>
                            <Trash2 color="#7a7a7a" />
                        </button>
                    </>
                }
            </div>
            {
                showDelete &&
                <div className="col-start-1 row-start-1 z-10 flex justify-end items-center gap-1">
                    <button className="grow" onClick={() => setShowDelete(false)}></button>
                    <button className="rounded-lg px-2 py-1 bg-black text-neutral-100" onClick={() => setShowDelete(false)}>Yes</button>
                    <button className="rounded-lg px-2 py-1 bg-black text-neutral-100" onClick={() => setShowDelete(false)}>No</button>
                </div>
            }
        </div>
    );
}

export default function NoteOverview() {
    const [notes, setNotes] = useState<Note[]>([]);
    const [filteredNotes, setFilteredNotes] = useState<Note[]>([]);
    const [selected, setSelected] = useState<number>(0);

    useEffect(() => {
        invoke("get_notes").then((result) => {
            setNotes(result as Note[]);
            setFilteredNotes(result as Note[]);
        });
    }, []);

    const handleKeyDown = (event: React.KeyboardEvent) => {
        if (event.key === 'ArrowUp') {
            event.preventDefault();
            setSelected((selected + filteredNotes.length - 1) % filteredNotes.length)
        } else if (event.key === 'ArrowDown') {
            event.preventDefault();
            setSelected((selected + filteredNotes.length + 1) % filteredNotes.length)
        }
    };

    const debounced = useDebouncedCallback(
        (search: string) => {
            if (search.length === 0) {
                setFilteredNotes(notes);
            }
            else {
                setFilteredNotes(notes.filter(n => n.content.substring(0, 20).toLocaleLowerCase().includes(search)))
            }
        },
        300
    );

    const textChanged = (e: React.ChangeEvent<HTMLInputElement>) => {
        debounced(e.target.value);
    }

    return (
        <div className="w-full h-full grid grid-rows-[auto_auto_1fr] gap-3 text-neutral-100 overflow-hidden">
            <div className="px-8">
                <input onChange={textChanged} onKeyDown={handleKeyDown} className="w-full bg-neutral-950 border-b-2 border-neutral-100 active:outline-none focus:outline-none" type="text" placeholder="Search ..." />
            </div>
            <div className="w-full flex justify-start items-center px-8">
                <h1 className="text-xl font-bold">Notiz</h1>
            </div>
            <div className="w-full overflow-y-auto px-8">
                {filteredNotes.map((note) =>
                    <ArticleInfo key={note.id} note={note} selected={filteredNotes[selected].id}></ArticleInfo>
                )}
            </div>
        </div>
    );
}