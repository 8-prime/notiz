export default async function setupAppWindow() {
    const appWindow = (await import('@tauri-apps/api/window')).getCurrentWindow()
    appWindow.show();
}