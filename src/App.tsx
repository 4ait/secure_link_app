import { useState, useEffect, useRef } from "react";
import {invoke} from "@tauri-apps/api/core";

function App() {

    const canvasRef = useRef<HTMLCanvasElement | null>(null);
    const [connectionState, setConnectionState] = useState('notConnected');

    const [error, setError] = useState(null);

    const [showTokenModal, setShowTokenModal] = useState(false);
    const [token, setToken] = useState('');
    const [savedToken, setSavedToken] = useState('');

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

        function createShader(gl, type, source) {
            const shader = gl.createShader(type);
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

        const program = gl.createProgram();
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

        function resizeCanvas() {
            canvas.width = window.innerWidth;
            canvas.height = window.innerHeight;
            gl.viewport(0, 0, canvas.width, canvas.height);
        }

        resizeCanvas();
        window.addEventListener('resize', resizeCanvas);

        function render(time) {
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

    const handleButtonClick = async () => {

        if (connectionState === "notConnected") {

            setConnectionState('connecting');

            try {

                await invoke("start")

                setConnectionState('connected');

                setError(null)


            } catch (e) {
                setError(e.toString())
                setConnectionState('notConnected');
            }

        } else if (connectionState === "connected") {


            try {

                await invoke("stop")

                setConnectionState('notConnected');

                setError(null)


            } catch (e) {
                setError(e.toString())
            }


        } else if (connectionState === "connecting") {

            try {

                await invoke("stop")

                setConnectionState('notConnected');

                setError(null)


            } catch (e) {
                setError(e.toString())
            }

        }
    };

    const handleTokenSave = () => {
        setSavedToken(token);
        setShowTokenModal(false);
        // Здесь можно добавить сохранение токена через invoke
        // await invoke("save_token", { token });
    };

    const getButtonText = () => {
        switch(connectionState) {
            case "connected": return "DISCONNECT";
            case "notConnected": return "CONNECT";
            case "connecting": return "CONNECTING...";
            default: return "CONNECT";
        }
    };

    return (
        <div style={styles.container}>

            <div style={{color: "white", position: "absolute", top: 20}}> {error}</div>

            <canvas
                ref={canvasRef}
                style={styles.canvas}
            />

            {/* Settings Button */}
            <button
                onClick={() => setShowTokenModal(true)}
                className="settings-button"
                style={styles.settingsButton}
            >
                <span style={styles.settingsIcon}>⚙</span>
            </button>

            {/* Settings Button */}
            <button
                onClick={async () => {

                    try {
                        await invoke("reinstall_service")
                        setError(null)

                    } catch (e) {
                        setError(e.toString())
                    }

                }}
                className="reinstall service"
                style={{
                    ...styles.settingsButton,
                    right: "70px"
                }}
            >
                <span style={styles.settingsIcon}>⚙</span>
            </button>

            {/* Main Button */}
            <button
                onClick={handleButtonClick}
                className={`cosmic-button ${connectionState}`}
                style={styles.button}
            >
                <span style={styles.buttonText}>{getButtonText()}</span>
            </button>

            {/* Token Modal */}
            {showTokenModal && (
                <div className="modal-overlay" style={styles.modalOverlay}>
                    <div className="modal-content" style={styles.modalContent}>
                        <div style={styles.modalHeader}>
                            <h2 style={styles.modalTitle}>API Configuration</h2>
                        </div>

                        <div style={styles.modalBody}>
                            <label style={styles.label}>API Token</label>
                            <input
                                type="password"
                                value={token}
                                onChange={(e) => setToken(e.target.value)}
                                placeholder="Enter your API token..."
                                className="cosmic-input"
                                style={styles.input}
                            />

                            {savedToken && (
                                <div style={styles.tokenStatus}>
                                    <span style={styles.tokenStatusText}>✓ Token saved</span>
                                </div>
                            )}
                        </div>

                        <div style={styles.modalActions}>
                            <button
                                onClick={() => setShowTokenModal(false)}
                                className="modal-button secondary"
                                style={styles.modalButtonSecondary}
                            >
                                Cancel
                            </button>
                            <button
                                onClick={handleTokenSave}
                                className="modal-button primary"
                                style={styles.modalButtonPrimary}
                            >
                                Save
                            </button>
                        </div>
                    </div>
                </div>
            )}

            <style jsx>{`
                .cosmic-button {
                    transition: all 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94);
                    backdrop-filter: blur(8px);
                    border: 2px solid rgba(220, 220, 240, 0.8);
                    box-shadow:
                            0 0 40px rgba(220, 220, 240, 0.4),
                            inset 0 1px 0 rgba(255, 255, 255, 0.3);
                    position: relative;
                    overflow: visible;
                }

                .cosmic-button:hover {
                    transform: scale(1.05);
                    border-color: rgba(240, 240, 255, 1.0);
                    box-shadow:
                            0 0 60px rgba(240, 240, 255, 0.6),
                            inset 0 1px 0 rgba(255, 255, 255, 0.4);
                }

                .cosmic-button:active {
                    transform: scale(0.98);
                }

                .cosmic-button.notConnected {
                    background: rgba(80, 80, 100, 0.4);
                    border-color: rgba(200, 200, 220, 0.8);
                    box-shadow:
                            0 0 40px rgba(200, 200, 220, 0.4),
                            inset 0 1px 0 rgba(255, 255, 255, 0.3);
                }

                .cosmic-button.connected {
                    background: rgba(100, 70, 80, 0.4);
                    border-color: rgba(240, 160, 180, 0.9);
                    box-shadow:
                            0 0 50px rgba(240, 160, 180, 0.5),
                            inset 0 1px 0 rgba(255, 255, 255, 0.3);
                }

                .cosmic-button.connected::before {
                    content: '';
                    position: absolute;
                    top: -20px;
                    left: -20px;
                    right: -20px;
                    bottom: -20px;
                    border: 1px solid rgba(240, 160, 180, 0.3);
                    border-radius: 50%;
                    animation: blackHoleRotation 4s linear infinite;
                }

                .cosmic-button.connected::after {
                    content: '';
                    position: absolute;
                    top: -35px;
                    left: -35px;
                    right: -35px;
                    bottom: -35px;
                    border: 1px solid rgba(240, 160, 180, 0.2);
                    border-top-color: rgba(240, 160, 180, 0.6);
                    border-radius: 50%;
                    animation: blackHoleRotation 6s linear infinite reverse;
                }

                .cosmic-button.connecting {
                    background: rgba(90, 90, 70, 0.4);
                    border-color: rgba(220, 220, 160, 0.9);
                    animation: connectingGlow 2s ease-in-out infinite;
                }

                .settings-button {
                    transition: all 0.3s cubic-bezier(0.25, 0.46, 0.45, 0.94);
                    backdrop-filter: blur(12px);
                    border: 1px solid rgba(180, 180, 200, 0.6);
                    box-shadow:
                        0 0 20px rgba(180, 180, 200, 0.3),
                        inset 0 1px 0 rgba(255, 255, 255, 0.2);
                }

                .settings-button:hover {
                    transform: scale(1.1) rotate(45deg);
                    border-color: rgba(200, 200, 220, 0.9);
                    box-shadow:
                        0 0 30px rgba(200, 200, 220, 0.5),
                        inset 0 1px 0 rgba(255, 255, 255, 0.3);
                }

                .modal-overlay {
                    animation: modalFadeIn 0.3s ease-out;
                }

                .modal-content {
                    animation: modalSlideIn 0.4s cubic-bezier(0.25, 0.46, 0.45, 0.94);
                    backdrop-filter: blur(20px);
                    border: 1px solid rgba(220, 220, 240, 0.3);
                    box-shadow:
                        0 0 80px rgba(100, 100, 200, 0.4),
                        inset 0 1px 0 rgba(255, 255, 255, 0.1);
                }

                .cosmic-input {
                    transition: all 0.3s ease;
                    border: 1px solid rgba(180, 180, 200, 0.4);
                    background: rgba(20, 20, 40, 0.6);
                    backdrop-filter: blur(10px);
                    box-shadow: inset 0 2px 10px rgba(0, 0, 0, 0.3);
                }

                .cosmic-input:focus {
                    outline: none;
                    border-color: rgba(200, 200, 240, 0.8);
                    box-shadow: 
                        inset 0 2px 10px rgba(0, 0, 0, 0.3),
                        0 0 20px rgba(200, 200, 240, 0.4);
                }

                .modal-button {
                    transition: all 0.3s cubic-bezier(0.25, 0.46, 0.45, 0.94);
                    backdrop-filter: blur(8px);
                    border: 1px solid rgba(180, 180, 200, 0.5);
                    cursor: pointer;
                    position: relative;
                    overflow: hidden;
                }

                .modal-button:hover {
                    transform: translateY(-2px);
                }

                .modal-button.primary {
                    background: rgba(70, 70, 120, 0.6);
                    border-color: rgba(150, 150, 200, 0.8);
                    box-shadow: 0 0 20px rgba(150, 150, 200, 0.3);
                }

                .modal-button.primary:hover {
                    background: rgba(80, 80, 140, 0.7);
                    box-shadow: 0 0 30px rgba(150, 150, 200, 0.5);
                }

                .modal-button.secondary {
                    background: rgba(60, 60, 80, 0.5);
                    border-color: rgba(120, 120, 140, 0.6);
                }

                .modal-button.secondary:hover {
                    background: rgba(70, 70, 90, 0.6);
                }

                @keyframes connectingGlow {
                    0%, 100% {
                        box-shadow:
                                0 0 50px rgba(220, 220, 160, 0.4),
                                inset 0 1px 0 rgba(255, 255, 255, 0.3);
                        border-color: rgba(220, 220, 160, 0.7);
                    }
                    50% {
                        box-shadow:
                                0 0 80px rgba(220, 220, 160, 0.7),
                                inset 0 1px 0 rgba(255, 255, 255, 0.4);
                        border-color: rgba(240, 240, 180, 1.0);
                    }
                }

                @keyframes blackHoleRotation {
                    0% { transform: rotate(0deg); }
                    100% { transform: rotate(360deg); }
                }

                @keyframes modalFadeIn {
                    from { opacity: 0; }
                    to { opacity: 1; }
                }

                @keyframes modalSlideIn {
                    from { 
                        opacity: 0;
                        transform: translateY(-30px) scale(0.95);
                    }
                    to { 
                        opacity: 1;
                        transform: translateY(0) scale(1);
                    }
                }
            `}</style>
        </div>
    );
}

const styles = {
    container: {
        position: 'relative',
        width: '100vw',
        height: '100vh',
        overflow: 'hidden',
        backgroundColor: '#000000',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center'
    },

    canvas: {
        position: 'absolute',
        top: 0,
        left: 0,
        width: '100%',
        height: '100%',
        zIndex: 1
    },

    settingsButton: {
        position: 'absolute',
        top: '30px',
        right: '30px',
        zIndex: 3,
        width: '50px',
        height: '50px',
        backgroundColor: 'rgba(40, 40, 60, 0.4)',
        border: 'none',
        borderRadius: '12px',
        cursor: 'pointer',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        fontFamily: 'system-ui, -apple-system, sans-serif'
    },

    settingsIcon: {
        color: '#e0e0e0',
        fontSize: '20px',
        textShadow: '0 0 10px rgba(255, 255, 255, 0.5)'
    },

    button: {
        position: 'relative',
        zIndex: 2,
        width: '140px',
        height: '140px',
        backgroundColor: 'rgba(40, 40, 50, 0.15)',
        border: 'none',
        borderRadius: '50%',
        cursor: 'pointer',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        fontFamily: 'system-ui, -apple-system, sans-serif'
    },

    buttonText: {
        color: '#f0f0f0',
        fontSize: '13px',
        fontWeight: '600',
        letterSpacing: '1.2px',
        textTransform: 'uppercase',
        textShadow: '0 0 15px rgba(255, 255, 255, 0.5)'
    },

    modalOverlay: {
        position: 'fixed',
        top: 0,
        left: 0,
        right: 0,
        bottom: 0,
        backgroundColor: 'rgba(0, 0, 0, 0.7)',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        zIndex: 10
    },

    modalContent: {
        background: 'rgba(20, 20, 35, 0.9)',
        borderRadius: '16px',
        padding: '0',
        width: '400px',
        maxWidth: '90vw',
        fontFamily: 'system-ui, -apple-system, sans-serif',
        overflow: 'hidden'
    },

    modalHeader: {
        padding: '24px 24px 16px 24px',
        borderBottom: '1px solid rgba(180, 180, 200, 0.2)'
    },

    modalTitle: {
        color: '#f0f0f0',
        fontSize: '20px',
        fontWeight: '600',
        margin: 0,
        textShadow: '0 0 10px rgba(255, 255, 255, 0.3)'
    },

    modalBody: {
        padding: '24px'
    },

    label: {
        display: 'block',
        color: '#d0d0d0',
        fontSize: '14px',
        fontWeight: '500',
        marginBottom: '8px',
        textShadow: '0 0 5px rgba(255, 255, 255, 0.2)'
    },

    input: {
        width: '100%',
        padding: '12px 16px',
        borderRadius: '8px',
        color: '#f0f0f0',
        fontSize: '14px',
        fontFamily: 'monospace',
        boxSizing: 'border-box'
    },

    tokenStatus: {
        marginTop: '12px',
        padding: '8px 12px',
        backgroundColor: 'rgba(80, 160, 80, 0.2)',
        border: '1px solid rgba(100, 200, 100, 0.4)',
        borderRadius: '6px'
    },

    tokenStatusText: {
        color: '#90ff90',
        fontSize: '12px',
        fontWeight: '500',
        textShadow: '0 0 8px rgba(144, 255, 144, 0.5)'
    },

    modalActions: {
        padding: '16px 24px 24px 24px',
        display: 'flex',
        gap: '12px',
        justifyContent: 'flex-end'
    },

    modalButtonPrimary: {
        padding: '10px 20px',
        borderRadius: '8px',
        color: '#f0f0f0',
        fontSize: '14px',
        fontWeight: '500',
        textShadow: '0 0 5px rgba(255, 255, 255, 0.3)'
    },

    modalButtonSecondary: {
        padding: '10px 20px',
        borderRadius: '8px',
        color: '#c0c0c0',
        fontSize: '14px',
        fontWeight: '500'
    }
};

export default App;