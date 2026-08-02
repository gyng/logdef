/**
 * The renderer's one primitive: a rounded rectangle with a vertical
 * gradient.
 *
 * Everything on screen — sky, terrain bands, tree silhouettes, floor
 * plates, room bodies, buffer fills, crew, the tower's legs — is one of
 * these. That is the entire trick behind the performance story: one
 * program, one instanced draw call per frame, no matter how much is on
 * screen. A few thousand instances cost one `bufferSubData` and one
 * `drawArraysInstanced`.
 *
 * Positions are in CSS pixels with the origin at the top left, because
 * the scene is laid out on the CPU anyway and a matrix stack would only
 * add a step to reason about.
 */

import { linkProgram } from "./gl";

/** Floats per instance. Keep in sync with the attribute layout below. */
const STRIDE = 16;

const VERTEX_SOURCE = `#version 300 es
precision highp float;

// Unit quad, expanded per instance.
layout(location = 0) in vec2 a_corner;

layout(location = 1) in vec4 i_rect;      // x, y, w, h  (pixels, top-left origin)
layout(location = 2) in vec4 i_colorTop;
layout(location = 3) in vec4 i_colorBottom;
layout(location = 4) in vec4 i_shape;     // radius, softness, rotation, _

uniform vec2 u_viewport;

out vec2 v_local;      // pixels from the rect's centre, before rotation
out vec2 v_halfSize;
out vec4 v_color;
out float v_radius;
out float v_softness;

void main() {
  vec2 size = i_rect.zw;
  vec2 local = (a_corner - 0.5) * size;

  // Rotate about the centre. Legs, vines, and bracing are all just
  // rectangles at an angle, so this is cheaper than a second program.
  float angle = i_shape.z;
  float c = cos(angle);
  float s = sin(angle);
  vec2 rotated = vec2(local.x * c - local.y * s, local.x * s + local.y * c);
  vec2 pos = i_rect.xy + size * 0.5 + rotated;

  // Pixel space -> clip space, y down.
  vec2 clip = vec2(
    (pos.x / u_viewport.x) * 2.0 - 1.0,
    1.0 - (pos.y / u_viewport.y) * 2.0
  );
  gl_Position = vec4(clip, 0.0, 1.0);

  v_halfSize = size * 0.5;
  v_local = local;
  v_color = mix(i_colorTop, i_colorBottom, a_corner.y);
  v_radius = i_shape.x;
  v_softness = i_shape.y;
}
`;

const FRAGMENT_SOURCE = `#version 300 es
precision highp float;

in vec2 v_local;
in vec2 v_halfSize;
in vec4 v_color;
in float v_radius;
in float v_softness;

out vec4 fragColor;

// Signed distance to a rounded box, in pixels.
float roundedBox(vec2 p, vec2 half_size, float radius) {
  vec2 q = abs(p) - half_size + radius;
  return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0) - radius;
}

void main() {
  // A plain rectangle takes the fast path with no analytic edge at all.
  // The SDF's half-pixel falloff is correct for an isolated shape but
  // wrong for tiling ones: two quads sharing an edge each contribute
  // ~50% coverage there, and the seam shows as a visible line across
  // the sky and the ground. Let the rasteriser handle straight edges.
  if (v_radius <= 0.0 && v_softness <= 0.0) {
    fragColor = v_color;
    return;
  }

  float radius = min(v_radius, min(v_halfSize.x, v_halfSize.y));
  float dist = roundedBox(v_local, v_halfSize, radius);
  // One-pixel analytic edge, widened by softness for glows and haze.
  float edge = 1.0 + v_softness;
  float coverage = 1.0 - smoothstep(-edge, 0.0, dist);
  if (coverage <= 0.0) discard;
  fragColor = vec4(v_color.rgb, v_color.a * coverage);
}
`;

export interface QuadOptions {
  /** Bottom gradient colour. Defaults to the top colour (flat fill). */
  colorBottom?: Color;
  /** Corner radius in pixels. */
  radius?: number;
  /** Extra edge softness in pixels, for glows and haze. */
  softness?: number;
  /** Clockwise rotation in radians, about the rectangle's centre. */
  rotation?: number;
}

/** Straight RGBA, each channel 0..1. */
export type Color = readonly [number, number, number, number];

export class QuadBatch {
  private readonly gl: WebGL2RenderingContext;
  private readonly program: WebGLProgram;
  private readonly vao: WebGLVertexArrayObject;
  private readonly instanceBuffer: WebGLBuffer;
  private readonly viewportLocation: WebGLUniformLocation;

  private data: Float32Array;
  private capacity: number;
  private count = 0;

  constructor(gl: WebGL2RenderingContext, initialCapacity = 4096) {
    this.gl = gl;
    this.capacity = initialCapacity;
    this.data = new Float32Array(this.capacity * STRIDE);
    this.program = linkProgram(gl, VERTEX_SOURCE, FRAGMENT_SOURCE, "quad");

    const viewport = gl.getUniformLocation(this.program, "u_viewport");
    if (!viewport) throw new Error("quad program is missing u_viewport");
    this.viewportLocation = viewport;

    const vao = gl.createVertexArray();
    const corners = gl.createBuffer();
    const instances = gl.createBuffer();
    if (!vao || !corners || !instances) throw new Error("could not allocate quad buffers");
    this.vao = vao;
    this.instanceBuffer = instances;

    gl.bindVertexArray(vao);

    // Unit quad as a triangle strip: (0,0) (1,0) (0,1) (1,1).
    gl.bindBuffer(gl.ARRAY_BUFFER, corners);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 0, 1, 0, 0, 1, 1, 1]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

    gl.bindBuffer(gl.ARRAY_BUFFER, instances);
    gl.bufferData(gl.ARRAY_BUFFER, this.data.byteLength, gl.DYNAMIC_DRAW);
    const bytes = STRIDE * 4;
    for (let location = 1; location <= 4; location += 1) {
      gl.enableVertexAttribArray(location);
      gl.vertexAttribPointer(location, 4, gl.FLOAT, false, bytes, (location - 1) * 16);
      gl.vertexAttribDivisor(location, 1);
    }

    gl.bindVertexArray(null);
    gl.bindBuffer(gl.ARRAY_BUFFER, null);
  }

  /** Drop everything queued. Call once at the top of a frame. */
  begin(): void {
    this.count = 0;
  }

  push(x: number, y: number, w: number, h: number, color: Color, options: QuadOptions = {}): void {
    // Fully transparent or degenerate quads still cost a vertex fetch.
    if (w <= 0 || h <= 0 || (color[3] <= 0 && (options.colorBottom?.[3] ?? 0) <= 0)) {
      return;
    }
    if (this.count === this.capacity) this.grow();

    const bottom = options.colorBottom ?? color;
    const at = this.count * STRIDE;
    const d = this.data;
    d[at] = x;
    d[at + 1] = y;
    d[at + 2] = w;
    d[at + 3] = h;
    d[at + 4] = color[0];
    d[at + 5] = color[1];
    d[at + 6] = color[2];
    d[at + 7] = color[3];
    d[at + 8] = bottom[0];
    d[at + 9] = bottom[1];
    d[at + 10] = bottom[2];
    d[at + 11] = bottom[3];
    d[at + 12] = options.radius ?? 0;
    d[at + 13] = options.softness ?? 0;
    d[at + 14] = options.rotation ?? 0;
    d[at + 15] = 0;
    this.count += 1;
  }

  /**
   * A thick line between two points, as one rotated quad. Used for the
   * tower's legs, hanging vines, and diagonal bracing.
   */
  pushLine(
    x1: number,
    y1: number,
    x2: number,
    y2: number,
    thickness: number,
    color: Color,
    options: QuadOptions = {},
  ): void {
    const dx = x2 - x1;
    const dy = y2 - y1;
    const length = Math.hypot(dx, dy);
    if (length < 0.01) return;
    this.push((x1 + x2) / 2 - length / 2, (y1 + y2) / 2 - thickness / 2, length, thickness, color, {
      radius: thickness / 2,
      ...options,
      rotation: Math.atan2(dy, dx),
    });
  }

  /** Upload and draw everything queued, in insertion order. */
  flush(viewportWidth: number, viewportHeight: number): void {
    if (this.count === 0) return;
    const gl = this.gl;

    gl.useProgram(this.program);
    gl.uniform2f(this.viewportLocation, viewportWidth, viewportHeight);

    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.instanceBuffer);
    gl.bufferSubData(gl.ARRAY_BUFFER, 0, this.data, 0, this.count * STRIDE);

    gl.enable(gl.BLEND);
    gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);
    gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, this.count);

    gl.bindVertexArray(null);
  }

  get queued(): number {
    return this.count;
  }

  dispose(): void {
    const gl = this.gl;
    gl.deleteBuffer(this.instanceBuffer);
    gl.deleteVertexArray(this.vao);
    gl.deleteProgram(this.program);
  }

  private grow(): void {
    this.capacity *= 2;
    const grown = new Float32Array(this.capacity * STRIDE);
    grown.set(this.data);
    this.data = grown;
    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.instanceBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, grown.byteLength, gl.DYNAMIC_DRAW);
  }
}
