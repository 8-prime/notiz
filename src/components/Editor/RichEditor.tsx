import './styles.scss'

// import Highlight from '@tiptap/extension-highlight'
// import Typography from '@tiptap/extension-typography'
import { Editor, EditorContent, useEditor } from '@tiptap/react'
import StarterKit from '@tiptap/starter-kit'
import { useEffect, useState } from 'react'
// import React from 'react'

export default ({ note, onChanges, onKeyDown }: { note: string, onChanges: (data: Editor) => void, onKeyDown: (event: KeyboardEvent) => void }) => {
    const editor = useEditor({
        extensions: [
            StarterKit,
            //   Highlight,
            //   Typography,
        ],
        content: note,
        editorProps: {
            handleKeyDown: (_, event: KeyboardEvent) => {
                onKeyDown(event)
            },
            attributes: {
                class: "bg-neutral-950 text-white w-full h-full p-1"
            }
        },
        onUpdate({ editor }) {
            onChanges(editor)
        }
    })

    const [init, setInit] = useState<boolean>(false);

    useEffect(() => {
        editor?.chain().focus()
    }, [])

    useEffect(() => {
        if (editor && !init && note.length > 0) {
            editor.commands.setContent(note)
            setInit(true)
        }
    }, [note])

    return (
        <EditorContent className='w-full h-full overflow-y-scroll' editor={editor} />
    )
}