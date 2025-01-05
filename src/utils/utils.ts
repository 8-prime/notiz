export default async function setupAppWindow() {
    const appWindow = (await import('@tauri-apps/api/window')).getCurrentWindow()
    appWindow.show();
}

export function getTitleFromText(text: string): string {
    const split = text.split('\n')
    if (split.length === 0) {
        return ""
    }
    return split[0].substring(0, Math.min(20, split[0].length))
}