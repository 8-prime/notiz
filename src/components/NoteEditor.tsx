import { invoke } from "@tauri-apps/api/core";
import { Dot, Save } from "lucide-react";
import React from "react";
import { useParams } from "react-router-dom";
import { useDebouncedCallback } from "use-debounce";
import { Note } from "../models/Note";


export default function NoteEditor() {
    const ref = React.useRef<HTMLTextAreaElement>(null);
    const [changes, setChanges] = React.useState(false);
    const [content, setContent] = React.useState<Note>({
        id: undefined,
        title: "",
        content: "",
        created_at: "",
        updated_at: ""
    })

    const { id } = useParams();

    React.useEffect(() => {
        if (id) {
            invoke("get_note", { id: id }).then((note) => {
                setContent(note as Note);
            });
        }
    }, [id]);

    React.useEffect(() => {
        if (ref.current) {
            ref.current.focus();
        }
    }, []);

    const debounced = useDebouncedCallback(
        (value: Note) => {
            console.log(value.content);

            setChanges(false);
            invoke("changes", { data: value }).then((update) => {
                console.log(update);
                setContent(update as Note);
            })
        },
        300
    );

    const textChanged = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
        setChanges(true);
        setContent({
            ...content,
            content: e.target.value
        });
        debounced({
            ...content,
            content: e.target.value
        });
    }

    return (
        <div className="grow flex flex-col items-center justify-start w-full p-4">
            <div className="w-full h-full overflow-hidden">
                <textarea
                    className="w-full h-full p-4 text-sm text-gray-800 bg-white rounded-lg  focus:outline-none resize-none"
                    ref={ref}
                    value={content.content}
                    onChange={textChanged}
                />
            </div>
            <div className="w-full flex justify-end items-center">
                {changes && <Dot strokeWidth={1} />}
                {!changes && <Save strokeWidth={1} />}
            </div>
        </div>
    );
}