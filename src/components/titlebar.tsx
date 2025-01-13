import { invoke } from "@tauri-apps/api/core";
import { X } from "lucide-react";

export default function Titlebar() {

    const handleMinimize = async () => {
        try {
            await invoke('minimize_window');
        } catch (error) {
            console.error('Failed to minimize window:', error);
        }
    };
    const handleMaximize = async () => {
        try {
            await invoke('maximize_window');
        } catch (error) {
            console.error('Failed to maximize window:', error);
        }
    };
    const handleClose = async () => {
        try {
            await invoke('close_window');
        } catch (error) {
            console.error('Failed to close window:', error);
        }
    };

    return (
        <div data-tauri-drag-region className="w-screen h-10 flex justify-end items-center">
            <button onClick={handleMinimize} >
                <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#7a7a7a" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><line x1="5" y1="18" x2="19" y2="18"></line></svg>
            </button>
            <button onClick={handleMaximize} >
                <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="#7a7a7a" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round"><line x1="5" y1="7" x2="19" y2="7"></line></svg>
            </button>
            <button onClick={handleClose} >
                <X color="#7a7a7a" />
            </button>
        </div>
    );
}