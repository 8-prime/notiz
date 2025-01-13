import { invoke } from "@tauri-apps/api/core";
import { Dot, Save } from "lucide-react";
import React from "react";
import { useParams } from "react-router-dom";
import { useDebouncedCallback } from "use-debounce";
import { Note } from "../models/Note";
import RichEditor from "./Editor/RichEditor";
import { Editor } from "@tiptap/react";
import { getTitleFromText } from "../utils/utils";


export default function NoteEditor() {
    const ref = React.useRef<HTMLTextAreaElement>(null);
    const [changes, setChanges] = React.useState(false);
    const [content, setContent] = React.useState<Note>({
        id: undefined,
        title: "",
        content: "",
        created_at: "",
        updated_at: "",
        favorite: false
    })
    const { id } = useParams();

    React.useEffect(() => {
        if (id) {
            invoke("get_note", { id: id }).then((note) => {
                console.log(note);
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
            setChanges(false);
            invoke("changes", { data: value }).then((update) => {
                setContent(update as Note);
            })
        },
        300
    );

    const textChanged = (editor: Editor) => {
        setChanges(true);
        const update = {
            ...content,
            content: editor.getHTML(),
            title: getTitleFromText(editor.getText()),
            updated_at: new Date().toDateString()
        }

        setContent(update)
        debounced(update);
    }

    const onKeyDown = (event: KeyboardEvent) => {
        if (event.ctrlKey && event.key === 'w') {
            event.preventDefault();
            setChanges(false);
            invoke("changes", { data: content }).then((update) => {
                setContent(update as Note);
                invoke('close_window');
            })
        }
    }

    return (
        <div className="h-full grid grid-rows-[1fr_auto] w-full gap-2 px-4 pb-4 overflow-hidden">
            <RichEditor note={content.content} onChanges={textChanged} onKeyDown={onKeyDown}></RichEditor>
            {/* <div className="w-full h-full">
                <textarea
                    className="w-full h-full px-4 text-sm text-neutral-100 bg-neutral-950 rounded-lg  focus:outline-none resize-none"
                    ref={ref}
                    value={content.content}
                    onChange={textChanged}
                    onKeyDown={onKeyDown}
                />
            </div> */}
            <div className="w-full flex justify-end items-center">
                {changes && <Dot strokeWidth={1} color="#7a7a7a" />}
                {!changes && <Save strokeWidth={1} color="#7a7a7a" />}
            </div>
        </div>
    );
}