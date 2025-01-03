import { invoke } from "@tauri-apps/api/core";
import { useEffect, useRef, useState } from "react";
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

function ArticleInfo({ note, selected, onDelete, onFavorite }: Readonly<{ note: Note, selected: string | undefined, onDelete: (id: string | undefined) => void, onFavorite: (note: Note) => void }>) {
    const [showDelete, setShowDelete] = useState<boolean>(false);
    const openArticle = (id: string | undefined) => {
        if (!id) return;
        invoke("open_article", { id: id })
    }

    const handleDelete = () => {
        setShowDelete(false);
        onDelete(note.id);
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
                        <button onClick={() => onFavorite(note)}>
                            <Star fill={note.favorite ? '#7a7a7a' : 'none'} color="#7a7a7a" />
                        </button>
                        <button onClick={() => setShowDelete(true)}>
                            <Trash2 color="#7a7a7a" />
                        </button>
                    </>
                }
            </div>
            {
                showDelete &&
                <div className="col-start-1 row-start-1 z-10 flex justify-end items-center gap-1 bg-neutral-800 hover:bg-neutral-700">
                    <button className="grow flex flex-row justify-start items-center" onClick={() => setShowDelete(false)}><p>Delete item?</p></button>
                    <button className="rounded-lg px-2 py-1 bg-black text-neutral-100" onClick={() => handleDelete()}>Yes</button>
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
    const [search, setSearch] = useState<string>('');

    const ref = useRef<HTMLInputElement>(null);

    useEffect(() => {
        invoke("get_notes").then((result) => {
            setNotes(result as Note[]);
            setFilteredNotes(result as Note[]);
        }).catch((err) => {
            console.error(err)
        });

        if (ref.current) {
            ref.current.focus();
        }
    }, []);

    const handleKeyDown = (event: React.KeyboardEvent) => {
        if (event.key === 'ArrowUp') {
            event.preventDefault();
            setSelected((selected + filteredNotes.length - 1) % filteredNotes.length)
        } else if (event.key === 'ArrowDown') {
            event.preventDefault();
            setSelected((selected + filteredNotes.length + 1) % filteredNotes.length)
        }
        if (event.key === 'Enter') {
            invoke("open_article", { id: filteredNotes[selected].id })
        }
        if (event.ctrlKey && event.key === 'w') {
            event.preventDefault();
            invoke('close_window');
        }
    };

    const updateSearch = (search: string, notes: Note[]) => {
        setSelected(0);
        if (search.length === 0) {
            setFilteredNotes(notes);
        }
        else {
            setSearch(search);
            setFilteredNotes(notes.filter(n => n.content.substring(0, 20).toLocaleLowerCase().includes(search)))
        }
    }


    const debounced = useDebouncedCallback(
        (search: string, notes: Note[]) => {
            updateSearch(search, notes)
        },
        300
    );

    const textChanged = (e: React.ChangeEvent<HTMLInputElement>) => {
        debounced(e.target.value, notes);
    }

    const onDelete = (id: string | undefined) => {
        if (!id) {
            return;
        }
        invoke("delete_note", { id: id })
            .then(() => {
                console.log("done deleting note");
                return invoke("get_notes")
            })
            .then((result) => {
                setNotes(result as Note[]);
                updateSearch(search, result as Note[])
            })
    }

    const onFavorite = (note: Note) => {
        note.favorite = !note.favorite
        invoke("changes", { data: note })
            .then(() => {
                console.log("done saving favorite");
                return invoke("get_notes")
            })
            .then((result) => {
                setNotes(result as Note[]);
                updateSearch(search, result as Note[])
            })
    }

    return (
        <div className="w-full h-full grid grid-rows-[auto_auto_1fr] gap-3 text-neutral-100 overflow-hidden">
            <div className="px-8">
                <input ref={ref} onChange={textChanged} onKeyDown={handleKeyDown} className="w-full bg-neutral-950 border-b-2 border-neutral-100 active:outline-none focus:outline-none" type="text" placeholder="Search ..." />
            </div>
            <div className="w-full flex justify-start items-center px-8">
                <h1 className="text-xl font-bold">Notiz</h1>
            </div>
            <div className="w-full overflow-y-auto px-8">
                {filteredNotes.map((note) =>
                    <ArticleInfo key={note.id} note={note} selected={filteredNotes[selected].id} onDelete={onDelete} onFavorite={onFavorite}></ArticleInfo>
                )}
            </div>
        </div>
    );
}