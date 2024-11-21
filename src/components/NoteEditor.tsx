import { invoke } from "@tauri-apps/api/core";
import { Dot, Save } from "lucide-react";
import React from "react";
import { useDebouncedCallback } from "use-debounce";

export default function NoteEditor() {

    const ref = React.useRef<HTMLTextAreaElement>(null);
    const [changes, setChanges] = React.useState(false);
    const [content, setContent] = React.useState("")

    React.useEffect(() => {
        if (ref.current) {
            ref.current.focus();
        }
    }, []);

    const debounced = useDebouncedCallback(
        // function
        (value) => {
            setChanges(false);
            invoke("changes", { value })
        },
        // delay in ms
        300
    );

    const textChanged = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
        setChanges(true);
        debounced(e.target.value);
        setContent(e.target.value);
    }

    return (
        <div className="grow flex flex-col items-center justify-start w-full p-4">
            <div className="w-full h-full overflow-hidden">
                <textarea
                    className="w-full h-full p-4 text-sm text-gray-800 bg-white rounded-lg  focus:outline-none resize-none"
                    ref={ref}
                    value={content}
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