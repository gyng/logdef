/**
 * Textured quads for the few parts of the renderer that are paintings
 * rather than geometry: full-frame washes, repeating paper, and atlas
 * sprites.
 *
 * Like `QuadBatch`, positions are CSS pixels from the top-left. UVs are
 * normalised and also use a top-left origin, so atlas coordinates can be
 * copied directly from an image editor without turning the y axis over.
 * Textures are loaded and owned by this batch; `dispose` releases every
 * texture as well as the program, VAO, and buffers.
 */

import { linkProgram } from "./gl";
import type { Color } from "./QuadBatch";

/** x, y, width, height in normalised texture coordinates. */
export type UvRect = readonly [number, number, number, number];

export interface TextureHandle {
  readonly url: string;
  readonly width: number;
  readonly height: number;
}

export interface TextureLoadOptions {
  /** Smooth filtering is right for paintings; nearest is useful for pixel atlases. */
  filter?: "linear" | "nearest";
  /** Build mip levels for clean downscaling. Defaults to true for linear textures. */
  mipmaps?: boolean;
  /** Set only for images served from another origin. */
  crossOrigin?: "anonymous" | "use-credentials";
}

export interface TextureQuadOptions {
  /** Normalised source rectangle, from the image's top-left. Defaults to the whole image. */
  uv?: UvRect;
  /** Straight RGBA multiplied over the sampled image. Defaults to opaque white. */
  tint?: Color;
  /** Clockwise rotation in radians around the rectangle's centre. */
  rotation?: number;
}

export interface CoverOptions extends TextureQuadOptions {
  /** Crop focus within the source, 0..1. Defaults to its centre. */
  focusX?: number;
  /** Crop focus within the source, 0..1. Defaults to its centre. */
  focusY?: number;
}

export interface RepeatOptions {
  /** Displayed width of one tile in pixels. Defaults to the image width. */
  tileWidth?: number;
  /** Displayed height of one tile in pixels. Defaults to the image height. */
  tileHeight?: number;
  /** Tile offset in display pixels. */
  offsetX?: number;
  /** Tile offset in display pixels. */
  offsetY?: number;
  tint?: Color;
}

const STRIDE = 13;
const WHITE: Color = [1, 1, 1, 1];
const FULL_UV: UvRect = [0, 0, 1, 1];

const VERTEX_SOURCE = `#version 300 es
precision highp float;

layout(location = 0) in vec2 a_corner;
layout(location = 1) in vec4 i_rect; // x, y, w, h in pixels
layout(location = 2) in vec4 i_uv;   // u, v, w, h from the top-left
layout(location = 3) in vec4 i_tint;
layout(location = 4) in float i_rotation;

uniform vec2 u_viewport;

out vec2 v_uv;
out vec4 v_tint;

void main() {
  vec2 local = (a_corner - vec2(0.5)) * i_rect.zw;
  float cosine = cos(i_rotation);
  float sine = sin(i_rotation);
  vec2 rotated = vec2(
    local.x * cosine - local.y * sine,
    local.x * sine + local.y * cosine
  );
  vec2 pos = i_rect.xy + i_rect.zw * 0.5 + rotated;
  vec2 clip = vec2(
    (pos.x / u_viewport.x) * 2.0 - 1.0,
    1.0 - (pos.y / u_viewport.y) * 2.0
  );
  gl_Position = vec4(clip, 0.0, 1.0);
  v_uv = i_uv.xy + a_corner * i_uv.zw;
  v_tint = i_tint;
}
`;

const FRAGMENT_SOURCE = `#version 300 es
precision highp float;

uniform sampler2D u_texture;

in vec2 v_uv;
in vec4 v_tint;

out vec4 fragColor;

void main() {
  fragColor = texture(u_texture, v_uv) * v_tint;
}
`;

export class TextureBatch {
  private readonly gl: WebGL2RenderingContext;
  private readonly program: WebGLProgram;
  private readonly vao: WebGLVertexArrayObject;
  private readonly cornerBuffer: WebGLBuffer;
  private readonly instanceBuffer: WebGLBuffer;
  private readonly viewportLocation: WebGLUniformLocation;
  private readonly textures = new Map<TextureHandle, WebGLTexture>();
  private readonly loads = new Map<string, Promise<TextureHandle>>();

  private data: Float32Array;
  private queuedTextures: TextureHandle[];
  private queuedRepeats: boolean[];
  private capacity: number;
  private count = 0;
  private disposed = false;

  constructor(gl: WebGL2RenderingContext, initialCapacity = 256) {
    this.gl = gl;
    this.capacity = Math.max(1, initialCapacity);
    this.data = new Float32Array(this.capacity * STRIDE);
    this.queuedTextures = Array.from<TextureHandle>({ length: this.capacity });
    this.queuedRepeats = Array.from<boolean>({ length: this.capacity });
    this.program = linkProgram(gl, VERTEX_SOURCE, FRAGMENT_SOURCE, "texture");

    const viewport = gl.getUniformLocation(this.program, "u_viewport");
    const sampler = gl.getUniformLocation(this.program, "u_texture");
    if (!viewport || !sampler) throw new Error("texture program is missing a uniform");
    this.viewportLocation = viewport;

    const vao = gl.createVertexArray();
    const corners = gl.createBuffer();
    const instances = gl.createBuffer();
    if (!vao || !corners || !instances) throw new Error("could not allocate texture buffers");
    this.vao = vao;
    this.cornerBuffer = corners;
    this.instanceBuffer = instances;

    gl.bindVertexArray(vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, corners);
    gl.bufferData(gl.ARRAY_BUFFER, new Float32Array([0, 0, 1, 0, 0, 1, 1, 1]), gl.STATIC_DRAW);
    gl.enableVertexAttribArray(0);
    gl.vertexAttribPointer(0, 2, gl.FLOAT, false, 0, 0);

    gl.bindBuffer(gl.ARRAY_BUFFER, instances);
    gl.bufferData(gl.ARRAY_BUFFER, this.data.byteLength, gl.DYNAMIC_DRAW);
    const bytes = STRIDE * Float32Array.BYTES_PER_ELEMENT;
    this.instanceAttribute(1, 4, bytes, 0);
    this.instanceAttribute(2, 4, bytes, 4);
    this.instanceAttribute(3, 4, bytes, 8);
    this.instanceAttribute(4, 1, bytes, 12);

    gl.useProgram(this.program);
    gl.uniform1i(sampler, 0);
    gl.bindVertexArray(null);
    gl.bindBuffer(gl.ARRAY_BUFFER, null);
  }

  /**
   * Load and upload an image URL. Identical URL/options share one owned
   * texture, including concurrent callers waiting on the same request.
   */
  load(url: string, options: TextureLoadOptions = {}): Promise<TextureHandle> {
    this.assertAlive();
    const filter = options.filter ?? "linear";
    const mipmaps = options.mipmaps ?? filter === "linear";
    const key = `${url}\u0000${filter}\u0000${String(mipmaps)}\u0000${options.crossOrigin ?? ""}`;
    const existing = this.loads.get(key);
    if (existing) return existing;

    const loading = this.loadImage(url, options.crossOrigin)
      .then((image) => this.upload(url, image, filter, mipmaps))
      .catch((error: unknown) => {
        this.loads.delete(key);
        throw error;
      });
    this.loads.set(key, loading);
    return loading;
  }

  /** Drop everything queued. Call once at the top of a frame. */
  begin(): void {
    this.assertAlive();
    this.count = 0;
  }

  /** Queue a textured rectangle. Quads draw in insertion order. */
  push(
    texture: TextureHandle,
    x: number,
    y: number,
    width: number,
    height: number,
    options: TextureQuadOptions = {},
  ): void {
    this.queue(
      texture,
      x,
      y,
      width,
      height,
      options.uv ?? FULL_UV,
      options.tint ?? WHITE,
      options.rotation ?? 0,
      false,
    );
  }

  /** Queue an atlas cell described in source-image pixels. */
  pushAtlas(
    texture: TextureHandle,
    x: number,
    y: number,
    width: number,
    height: number,
    sourceX: number,
    sourceY: number,
    sourceWidth: number,
    sourceHeight: number,
    tint: Color = WHITE,
    rotation = 0,
  ): void {
    this.push(texture, x, y, width, height, {
      uv: [
        sourceX / texture.width,
        sourceY / texture.height,
        sourceWidth / texture.width,
        sourceHeight / texture.height,
      ],
      tint,
      rotation,
    });
  }

  /**
   * Queue a horizontal atlas cell stretched between two endpoints.
   * The source's left edge lands at (x1, y1), and its right edge at (x2, y2).
   */
  pushAtlasSegment(
    texture: TextureHandle,
    x1: number,
    y1: number,
    x2: number,
    y2: number,
    thickness: number,
    sourceX: number,
    sourceY: number,
    sourceWidth: number,
    sourceHeight: number,
    tint: Color = WHITE,
  ): void {
    const dx = x2 - x1;
    const dy = y2 - y1;
    const length = Math.hypot(dx, dy);
    if (length <= 0 || thickness <= 0) return;
    this.pushAtlas(
      texture,
      (x1 + x2 - length) / 2,
      (y1 + y2 - thickness) / 2,
      length,
      thickness,
      sourceX,
      sourceY,
      sourceWidth,
      sourceHeight,
      tint,
      Math.atan2(dy, dx),
    );
  }

  /** Fill a rectangle without distortion, cropping the source around a focus point. */
  pushCover(
    texture: TextureHandle,
    x: number,
    y: number,
    width: number,
    height: number,
    options: CoverOptions = {},
  ): void {
    if (width <= 0 || height <= 0) return;
    const sourceAspect = texture.width / texture.height;
    const targetAspect = width / height;
    const focusX = clamp01(options.focusX ?? 0.5);
    const focusY = clamp01(options.focusY ?? 0.5);
    let uv: UvRect;
    if (sourceAspect > targetAspect) {
      const visible = targetAspect / sourceAspect;
      uv = [clamp(focusX - visible / 2, 0, 1 - visible), 0, visible, 1];
    } else {
      const visible = sourceAspect / targetAspect;
      uv = [0, clamp(focusY - visible / 2, 0, 1 - visible), 1, visible];
    }
    this.push(texture, x, y, width, height, {
      uv,
      tint: options.tint,
      rotation: options.rotation,
    });
  }

  /** Cover the whole viewport. A convenience for title cards and painted skies. */
  pushFullscreenCover(
    texture: TextureHandle,
    viewportWidth: number,
    viewportHeight: number,
    options: CoverOptions = {},
  ): void {
    this.pushCover(texture, 0, 0, viewportWidth, viewportHeight, options);
  }

  /** Repeat an image over a rectangle, useful for paper grain and wash tiles. */
  pushRepeat(
    texture: TextureHandle,
    x: number,
    y: number,
    width: number,
    height: number,
    options: RepeatOptions = {},
  ): void {
    const tileWidth = options.tileWidth ?? texture.width;
    const tileHeight = options.tileHeight ?? texture.height;
    if (tileWidth <= 0 || tileHeight <= 0) throw new Error("texture tile size must be positive");
    const offsetX = options.offsetX ?? 0;
    const offsetY = options.offsetY ?? 0;
    this.queue(
      texture,
      x,
      y,
      width,
      height,
      [offsetX / tileWidth, offsetY / tileHeight, width / tileWidth, height / tileHeight],
      options.tint ?? WHITE,
      0,
      true,
    );
  }

  /** Repeat an image over the full viewport. */
  pushFullscreenRepeat(
    texture: TextureHandle,
    viewportWidth: number,
    viewportHeight: number,
    options: RepeatOptions = {},
  ): void {
    this.pushRepeat(texture, 0, 0, viewportWidth, viewportHeight, options);
  }

  /** Upload and draw everything queued, preserving alpha-sensitive insertion order. */
  flush(viewportWidth: number, viewportHeight: number): void {
    this.assertAlive();
    if (this.count === 0) return;
    const gl = this.gl;
    gl.useProgram(this.program);
    gl.uniform2f(this.viewportLocation, viewportWidth, viewportHeight);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindVertexArray(this.vao);
    gl.bindBuffer(gl.ARRAY_BUFFER, this.instanceBuffer);
    gl.enable(gl.BLEND);
    gl.blendFuncSeparate(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA, gl.ONE, gl.ONE_MINUS_SRC_ALPHA);

    // WebGL2 has no base-instance draw. Upload each consecutive texture
    // run at offset zero; this preserves translucent draw order while
    // still drawing every run in one instanced call.
    for (let start = 0; start < this.count;) {
      const handle = this.queuedTextures[start]!;
      const repeat = this.queuedRepeats[start]!;
      let end = start + 1;
      while (
        end < this.count &&
        this.queuedTextures[end] === handle &&
        this.queuedRepeats[end] === repeat
      ) {
        end += 1;
      }
      const owned = this.textures.get(handle);
      if (!owned) throw new Error(`texture is not owned by this batch: ${handle.url}`);
      gl.bindTexture(gl.TEXTURE_2D, owned);
      const wrap = repeat ? gl.REPEAT : gl.CLAMP_TO_EDGE;
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, wrap);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, wrap);
      gl.bufferSubData(gl.ARRAY_BUFFER, 0, this.data.subarray(start * STRIDE, end * STRIDE));
      gl.drawArraysInstanced(gl.TRIANGLE_STRIP, 0, 4, end - start);
      start = end;
    }

    gl.bindTexture(gl.TEXTURE_2D, null);
    gl.bindVertexArray(null);
  }

  get queued(): number {
    return this.count;
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    const gl = this.gl;
    for (const texture of this.textures.values()) gl.deleteTexture(texture);
    this.textures.clear();
    this.loads.clear();
    gl.deleteBuffer(this.cornerBuffer);
    gl.deleteBuffer(this.instanceBuffer);
    gl.deleteVertexArray(this.vao);
    gl.deleteProgram(this.program);
    this.count = 0;
  }

  private queue(
    texture: TextureHandle,
    x: number,
    y: number,
    width: number,
    height: number,
    uv: UvRect,
    tint: Color,
    rotation: number,
    repeat: boolean,
  ): void {
    this.assertAlive();
    if (!this.textures.has(texture))
      throw new Error(`texture is not owned by this batch: ${texture.url}`);
    if (width <= 0 || height <= 0 || tint[3] <= 0) return;
    if (this.count === this.capacity) this.grow();
    const at = this.count * STRIDE;
    this.data.set([x, y, width, height, uv[0], uv[1], uv[2], uv[3], ...tint, rotation], at);
    this.queuedTextures[this.count] = texture;
    this.queuedRepeats[this.count] = repeat;
    this.count += 1;
  }

  private async loadImage(
    url: string,
    crossOrigin: TextureLoadOptions["crossOrigin"],
  ): Promise<HTMLImageElement> {
    const image = new Image();
    image.decoding = "async";
    if (crossOrigin) image.crossOrigin = crossOrigin;
    const loaded = new Promise<void>((resolve, reject) => {
      image.addEventListener("load", () => resolve(), { once: true });
      image.addEventListener("error", () => reject(new Error(`could not load texture: ${url}`)), {
        once: true,
      });
    });
    image.src = url;
    await loaded;
    return image;
  }

  private upload(
    url: string,
    image: HTMLImageElement,
    filter: NonNullable<TextureLoadOptions["filter"]>,
    mipmaps: boolean,
  ): TextureHandle {
    this.assertAlive();
    const gl = this.gl;
    const texture = gl.createTexture();
    if (!texture) throw new Error(`could not allocate texture: ${url}`);
    gl.bindTexture(gl.TEXTURE_2D, texture);
    const previousFlip = Boolean(gl.getParameter(gl.UNPACK_FLIP_Y_WEBGL));
    // Screen positions and atlas UVs both use a top-left origin. HTML images
    // already arrive in row order for that convention; WebGL's usual flip is
    // only needed by bottom-left texture-coordinate renderers and would turn
    // every room, frame component and landscape upside down here.
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, 0);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, image);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, previousFlip ? 1 : 0);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
    if (filter === "nearest") {
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
    } else {
      gl.texParameteri(
        gl.TEXTURE_2D,
        gl.TEXTURE_MIN_FILTER,
        mipmaps ? gl.LINEAR_MIPMAP_LINEAR : gl.LINEAR,
      );
      gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    }
    if (mipmaps) gl.generateMipmap(gl.TEXTURE_2D);
    gl.bindTexture(gl.TEXTURE_2D, null);

    const handle: TextureHandle = Object.freeze({
      url,
      width: image.naturalWidth,
      height: image.naturalHeight,
    });
    this.textures.set(handle, texture);
    return handle;
  }

  private instanceAttribute(
    location: number,
    size: number,
    stride: number,
    floatOffset: number,
  ): void {
    const gl = this.gl;
    gl.enableVertexAttribArray(location);
    gl.vertexAttribPointer(
      location,
      size,
      gl.FLOAT,
      false,
      stride,
      floatOffset * Float32Array.BYTES_PER_ELEMENT,
    );
    gl.vertexAttribDivisor(location, 1);
  }

  private grow(): void {
    this.capacity *= 2;
    const grown = new Float32Array(this.capacity * STRIDE);
    grown.set(this.data);
    this.data = grown;
    this.queuedTextures.length = this.capacity;
    this.queuedRepeats.length = this.capacity;
    const gl = this.gl;
    gl.bindBuffer(gl.ARRAY_BUFFER, this.instanceBuffer);
    gl.bufferData(gl.ARRAY_BUFFER, grown.byteLength, gl.DYNAMIC_DRAW);
  }

  private assertAlive(): void {
    if (this.disposed) throw new Error("texture batch has been disposed");
  }
}

function clamp(value: number, low: number, high: number): number {
  return Math.min(high, Math.max(low, value));
}

function clamp01(value: number): number {
  return clamp(value, 0, 1);
}
