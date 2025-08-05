import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";
import backgroundUrl from './assets/background.png'

type ConnectionState = 'notConnected' | 'connecting' | 'connected';

function App() {
    const contextMenuRef = useRef<HTMLDivElement | null>(null);
    const [connectionState, setConnectionState] = useState<ConnectionState>('notConnected');
    const [error, setError] = useState<string | null>(null);
    const [showTokenModal, setShowTokenModal] = useState<boolean>(false);
    const [showContextMenu, setShowContextMenu] = useState<boolean>(false);
    const [token, setToken] = useState<string | null>(null);
    const [savedToken, setSavedToken] = useState<string>('');
    const [pasteSuccess, setPasteSuccess] = useState<boolean>(false);
    const pollingIntervalRef = useRef<number | null>(null);
    const pasteTimeoutRef = useRef<number | null>(null);

    // Polling function to check service status
    const checkServiceStatus = async (): Promise<void> => {
        try {

            const currentState: 'Running' | 'Pending' | 'Stopped' = await invoke("current_state");

            if (currentState === 'Running') {
                setConnectionState('connected');
                setError(null);
            }else if (currentState === 'Pending')
            {
                setConnectionState('connecting');
                setError(null);
            }else if (currentState === 'Stopped') {
                setConnectionState('notConnected');
            }

        } catch (e) {
            setError(String(e));
        }
    };

    // Start polling when component mounts
    useEffect(() => {
        // Initial status check
        checkServiceStatus();

        // Start polling every 500 ms
        pollingIntervalRef.current = setInterval(checkServiceStatus, 500);

        // Cleanup on unmount
        return () => {
            if (pollingIntervalRef.current) {
                clearInterval(pollingIntervalRef.current);
                pollingIntervalRef.current = null;
            }
            if (pasteTimeoutRef.current) {
                clearTimeout(pasteTimeoutRef.current);
                pasteTimeoutRef.current = null;
            }
        };
    }, []);

    useEffect(() => {

        (async () => {

            const token: string | null = await invoke("get_auth_token");

            token && setToken(token)

        })()

    }, []);

    // Close context menu when clicking outside
    useEffect(() => {
        const handleClickOutside = (event: MouseEvent) => {
            if (contextMenuRef.current && !contextMenuRef.current.contains(event.target as Node)) {
                setShowContextMenu(false);
            }
        };

        if (showContextMenu) {
            document.addEventListener('mousedown', handleClickOutside);
        }

        return () => {
            document.removeEventListener('mousedown', handleClickOutside);
        };
    }, [showContextMenu]);

    const handleButtonClick = async (): Promise<void> => {

        if (connectionState === "notConnected") {

            try {

                if (!token) {
                    setShowTokenModal(true)
                    return
                }

                setConnectionState('connecting');
                await invoke("start");
                setConnectionState('connected');
                setError(null);

            } catch (e) {
                setError(String(e));
                setConnectionState('notConnected');
            }

        } else if (connectionState === "connected") {
            try {
                await invoke("stop");
                setConnectionState('notConnected');
                setError(null);
            } catch (e) {
                setError(String(e));
            }
        } else if (connectionState === "connecting") {
            try {
                await invoke("stop");
                setConnectionState('notConnected');
                setError(null);
            } catch (e) {
                setError(String(e));
            }
        }
    };

    const handleSettingsClick = (): void => {
        setShowContextMenu(!showContextMenu);
    };

    const handleTokenMenuClick = (): void => {
        setShowTokenModal(true);
        setShowContextMenu(false);
    };

    const handleServiceLogClick = async (): Promise<void> => {
        setShowContextMenu(false);

        try {
            // Get service log from backend
            const logData = await invoke("get_service_log") as string;

            // Create blob and download
            const blob = new Blob([logData], { type: 'text/plain' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `service-log-${new Date().toISOString().split('T')[0]}.txt`;
            document.body.appendChild(a);
            a.click();
            document.body.removeChild(a);
            URL.revokeObjectURL(url);
        } catch (e) {
            setError('Failed to export service log');
        }
    };

    const handlePasteClick = async (): Promise<void> => {
        try {
            // Check if clipboard API is available
            if (!navigator.clipboard || !navigator.clipboard.readText) {
                setError('Clipboard API not supported');
                return;
            }

            // Request clipboard permission and read text
            const text = await navigator.clipboard.readText();

            if (text.trim()) {
                setToken(text.trim());

                // Show success state
                setPasteSuccess(true);

                // Reset after 2 seconds
                if (pasteTimeoutRef.current) {
                    clearTimeout(pasteTimeoutRef.current);
                }
                pasteTimeoutRef.current = setTimeout(() => {
                    setPasteSuccess(false);
                }, 2000);
            } else {
                setError('Clipboard is empty');
            }
        } catch (e) {
            console.error('Clipboard error:', e);
            setError('Failed to read clipboard. Make sure the app has permission.');
        }
    };

    const handleTokenSave = async (): Promise<void> => {

        if (!token) {
            return
        }

        try {
            await invoke("update_auth_token", { authToken: token });
            setSavedToken(token);
            setShowTokenModal(false);
            setError(null);
        } catch (e) {
            setError(String(e));
        }
    };

    const getButtonText = (): string => {
        switch(connectionState) {
            case "connected": return "DISCONNECT";
            case "notConnected": return "CONNECT";
            case "connecting": return "CONNECTING...";
            default: return "CONNECT";
        }
    };

    const getButtonStateClass = (): string => {
        switch(connectionState) {
            case "connected": return "connected";
            case "notConnected": return "not-connected";
            case "connecting": return "connecting";
            default: return "not-connected";
        }
    };

    return (
        <div className="app-container" style={{backgroundImage: `url(${backgroundUrl})`}}>
            <div className="error-display">{error}</div>

            {/* Settings Button with Context Menu */}
            <div className="settings-container">
                <button
                    onClick={handleSettingsClick}
                    className="settings-button"
                >
                    <span className="settings-icon">⚙</span>
                </button>

                {/* Context Menu */}
                {showContextMenu && (
                    <div ref={contextMenuRef} className="context-menu">
                        <button
                            onClick={handleTokenMenuClick}
                            className="context-menu-item"
                        >
                            Настройка подключения
                        </button>
                        <button
                            onClick={handleServiceLogClick}
                            className="context-menu-item"
                        >
                            Лог службы
                        </button>
                    </div>
                )}
            </div>

            {/* Main Button */}
            <button
                onClick={handleButtonClick}
                className={`main-button ${getButtonStateClass()}`}
            >
                <span className="button-text">{getButtonText()}</span>
                {connectionState === 'connected' && (
                    <>
                        <div className="black-hole-ring-1"></div>
                        <div className="black-hole-ring-2"></div>
                    </>
                )}
            </button>

            {/* Token Modal */}
            {showTokenModal && (
                <div className="modal-overlay">
                    <div className="modal-content">
                        <div className="modal-header">
                            <h2 className="modal-title">Настройка подключения</h2>
                        </div>

                        <div className="modal-body">
                            <label className="modal-label">Токен</label>
                            <div className="token-input-container">
                                <input
                                    type="text"
                                    value={token || ''}
                                    onChange={(e: React.ChangeEvent<HTMLInputElement>) => setToken(e.target.value)}
                                    placeholder="Введите ваш токен..."
                                    className="modal-input"
                                />
                                <button
                                    onClick={handlePasteClick}
                                    className={`paste-button ${pasteSuccess ? 'paste-success' : ''}`}
                                    title="Вставьте из буфера обмена"
                                >
                                    {pasteSuccess ? '✓' : '⎘'}
                                </button>
                            </div>

                            {savedToken && (
                                <div className="token-status">
                                    <span className="token-status-text">✓ Токен сохранен</span>
                                </div>
                            )}
                        </div>

                        <div className="modal-actions">
                            <button
                                onClick={() => setShowTokenModal(false)}
                                className="modal-button modal-button-secondary"
                            >
                                Cancel
                            </button>
                            <button
                                onClick={handleTokenSave}
                                className="modal-button modal-button-primary"
                            >
                                Save
                            </button>
                        </div>
                    </div>
                </div>
            )}
        </div>
    );
}

export default App;