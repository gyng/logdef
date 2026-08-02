/**
 * Thin WebGL2 helpers. Not a graphics library — just the four or five
 * things needed to compile a program and push a buffer, with errors
 * that say what went wrong instead of leaving you with a black canvas.
 */

export function createContext(canvas: HTMLCanvasElement): WebGL2RenderingContext {
  const gl = canvas.getContext("webgl2", {
    alpha: false,
    antialias: true,
    depth: false,
    // The cross-section is read, not scrubbed; preserving the buffer
    // costs bandwidth for nothing.
    preserveDrawingBuffer: false,
    powerPreference: "low-power",
  });
  if (!gl) {
    throw new Error("WebGL2 is unavailable in this browser");
  }
  return gl;
}

export function compileShader(
  gl: WebGL2RenderingContext,
  kind: number,
  source: string,
  label: string,
): WebGLShader {
  const shader = gl.createShader(kind);
  if (!shader) throw new Error(`could not allocate ${label} shader`);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const log = gl.getShaderInfoLog(shader) ?? "(no log)";
    gl.deleteShader(shader);
    throw new Error(`${label} shader failed to compile:\n${log}`);
  }
  return shader;
}

export function linkProgram(
  gl: WebGL2RenderingContext,
  vertexSource: string,
  fragmentSource: string,
  label: string,
): WebGLProgram {
  const vertex = compileShader(gl, gl.VERTEX_SHADER, vertexSource, `${label} vertex`);
  const fragment = compileShader(gl, gl.FRAGMENT_SHADER, fragmentSource, `${label} fragment`);
  const program = gl.createProgram();
  if (!program) throw new Error(`could not allocate ${label} program`);
  gl.attachShader(program, vertex);
  gl.attachShader(program, fragment);
  gl.linkProgram(program);
  // The shaders are owned by the program once linked.
  gl.deleteShader(vertex);
  gl.deleteShader(fragment);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    const log = gl.getProgramInfoLog(program) ?? "(no log)";
    gl.deleteProgram(program);
    throw new Error(`${label} program failed to link:\n${log}`);
  }
  return program;
}

/**
 * Size the drawing buffer to the element's CSS box at the device pixel
 * ratio, capped so a 4x-DPI display doesn't quadruple the fill cost for
 * a cross-section made of flat rectangles.
 */
export function resizeToDisplay(canvas: HTMLCanvasElement, maxRatio = 2): boolean {
  const ratio = Math.min(window.devicePixelRatio || 1, maxRatio);
  const width = Math.max(1, Math.round(canvas.clientWidth * ratio));
  const height = Math.max(1, Math.round(canvas.clientHeight * ratio));
  if (canvas.width === width && canvas.height === height) return false;
  canvas.width = width;
  canvas.height = height;
  return true;
}
