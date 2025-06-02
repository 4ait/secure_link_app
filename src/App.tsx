import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type ConnectionState = 'notConnected' | 'connecting' | 'connected';

function App() {
    const canvasRef = useRef<HTMLCanvasElement | null>(null);
    const contextMenuRef = useRef<HTMLDivElement | null>(null);
    const [connectionState, setConnectionState] = useState<ConnectionState>('notConnected');
    const [error, setError] = useState<string | null>(null);
    const [showTokenModal, setShowTokenModal] = useState<boolean>(false);
    const [showContextMenu, setShowContextMenu] = useState<boolean>(false);
    const [token, setToken] = useState<string>('');
    const [savedToken, setSavedToken] = useState<string>('');

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

    useEffect(() => {
        const canvas = canvasRef.current;
        if (!canvas) return;

        const gl = canvas.getContext('webgl2') || canvas.getContext('webgl');
        if (!gl) return;

        // Vertex shader
        const vertexShaderSource = `
      attribute vec2 a_position;
      void main() {
        gl_Position = vec4(a_position, 0.0, 1.0);
      }
    `;

        // Fragment shader - глубокий космос
        const fragmentShaderSource = `
      precision highp float;
      uniform float u_time;
      uniform vec2 u_resolution;
      
      float random(vec2 st) {
        return fract(sin(dot(st.xy, vec2(12.9898,78.233))) * 43758.5453123);
      }
      
      float noise(vec2 st) {
        vec2 i = floor(st);
        vec2 f = fract(st);
        float a = random(i);
        float b = random(i + vec2(1.0, 0.0));
        float c = random(i + vec2(0.0, 1.0));
        float d = random(i + vec2(1.0, 1.0));
        vec2 u = f * f * (3.0 - 2.0 * f);
        return mix(a, b, u.x) + (c - a)* u.y * (1.0 - u.x) + (d - b) * u.x * u.y;
      }
      
      float fbm(vec2 st) {
        float value = 0.0;
        float amplitude = 0.5;
        for (int i = 0; i < 5; i++) {
          value += amplitude * noise(st);
          st *= 2.0;
          amplitude *= 0.5;
        }
        return value;
      }
      
      vec3 nebula(vec2 uv) {
        vec2 st = uv * 3.0 + u_time * 0.1;
        float n = fbm(st);
        
        vec3 color1 = vec3(0.1, 0.0, 0.2);  // темно-фиолетовый
        vec3 color2 = vec3(0.2, 0.0, 0.4);  // фиолетовый
        vec3 color3 = vec3(0.0, 0.0, 0.1);  // почти черный
        
        vec3 nebColor = mix(color3, color1, n);
        nebColor = mix(nebColor, color2, pow(n, 2.0));
        
        return nebColor * 0.3;
      }
      
      float stars(vec2 uv) {
        vec2 st = uv * 150.0;
        vec2 ipos = floor(st);
        vec2 fpos = fract(st);
        
        float rnd = random(ipos);
        float size = 0.0005 + rnd * 0.002;
        float brightness = pow(rnd, 3.0);
        
        if (rnd > 0.96) {
          float dist = length(fpos - 0.5);
          return brightness * (1.0 - smoothstep(0.0, size, dist));
        }
        return 0.0;
      }
      
      void main() {
        vec2 uv = (gl_FragCoord.xy - 0.5 * u_resolution.xy) / u_resolution.y;
        
        // Базовый черный космос
        vec3 color = vec3(0.0, 0.0, 0.02);
        
        // Добавляем туманности
        color += nebula(uv);
        
        // Добавляем звезды
        color += vec3(stars(uv));
        
        // Добавляем дистантное свечение
        float dist = length(uv);
        color += vec3(0.02, 0.01, 0.05) * (1.0 / (1.0 + dist * 2.0));
        
        gl_FragColor = vec4(color, 1.0);
      }
    `;

        function createShader(gl: WebGLRenderingContext | WebGL2RenderingContext, type: number, source: string): WebGLShader | null {
            const shader = gl.createShader(type);
            if (!shader) return null;

            gl.shaderSource(shader, source);
            gl.compileShader(shader);
            if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
                console.error(gl.getShaderInfoLog(shader));
                gl.deleteShader(shader);
                return null;
            }
            return shader;
        }

        const vertexShader = createShader(gl, gl.VERTEX_SHADER, vertexShaderSource);
        const fragmentShader = createShader(gl, gl.FRAGMENT_SHADER, fragmentShaderSource);

        if (!vertexShader || !fragmentShader) return;

        const program = gl.createProgram();
        if (!program) return;

        gl.attachShader(program, vertexShader);
        gl.attachShader(program, fragmentShader);
        gl.linkProgram(program);

        if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
            console.error(gl.getProgramInfoLog(program));
            return;
        }

        const positionAttributeLocation = gl.getAttribLocation(program, 'a_position');
        const timeUniformLocation = gl.getUniformLocation(program, 'u_time');
        const resolutionUniformLocation = gl.getUniformLocation(program, 'u_resolution');

        const positionBuffer = gl.createBuffer();
        gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
        gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([
            -1, -1,
            1, -1,
            -1,  1,
            -1,  1,
            1, -1,
            1,  1,
        ]), gl.STATIC_DRAW);

        function resizeCanvas(): void {
            if (!canvas || !gl) return;
            canvas.width = window.innerWidth;
            canvas.height = window.innerHeight;
            gl.viewport(0, 0, canvas.width, canvas.height);
        }

        resizeCanvas();
        window.addEventListener('resize', resizeCanvas);

        function render(time: number): void {
            if (!canvas || !gl) return;

            gl.clearColor(0, 0, 0, 1);
            gl.clear(gl.COLOR_BUFFER_BIT);

            gl.useProgram(program);
            gl.bindBuffer(gl.ARRAY_BUFFER, positionBuffer);
            gl.enableVertexAttribArray(positionAttributeLocation);
            gl.vertexAttribPointer(positionAttributeLocation, 2, gl.FLOAT, false, 0, 0);

            gl.uniform1f(timeUniformLocation, time * 0.001);
            gl.uniform2f(resolutionUniformLocation, canvas.width, canvas.height);

            gl.drawArrays(gl.TRIANGLES, 0, 6);
            requestAnimationFrame(render);
        }

        requestAnimationFrame(render);

        return () => {
            window.removeEventListener('resize', resizeCanvas);
        };
    }, []);

    const handleButtonClick = async (): Promise<void> => {
        if (connectionState === "notConnected") {
            setConnectionState('connecting');

            try {
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

    const handleTokenSave = async (): Promise<void> => {
        try {
            await invoke("update_auth_token", { auth_token: token });
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
        <div className="app-container">
            <div className="error-display">{error}</div>

            <canvas
                ref={canvasRef}
                className="app-canvas"
            />

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
                            API Token
                        </button>
                        <button
                            onClick={handleServiceLogClick}
                            className="context-menu-item"
                        >
                            Service Log
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
                            <h2 className="modal-title">API Configuration</h2>
                        </div>

                        <div className="modal-body">
                            <label className="modal-label">API Token</label>
                            <input
                                type="password"
                                value={token}
                                onChange={(e: React.ChangeEvent<HTMLInputElement>) => setToken(e.target.value)}
                                placeholder="Enter your API token..."
                                className="modal-input"
                            />

                            {savedToken && (
                                <div className="token-status">
                                    <span className="token-status-text">✓ Token saved</span>
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