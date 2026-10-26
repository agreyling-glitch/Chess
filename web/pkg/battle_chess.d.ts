/* tslint:disable */
/* eslint-disable */

/**
 * si5 keeps names only; index records and game blobs are supplied as bounded
 * chunks from File.slice(), preserving 47-bit offsets on wasm32.
 */
export class Scid5Reader {
    free(): void;
    [Symbol.dispose](): void;
    decode_game(record: Uint8Array, blob: Uint8Array): string;
    index_entries(bytes: Uint8Array): string;
    constructor(names: Uint8Array);
}

export class ScidImport {
    free(): void;
    [Symbol.dispose](): void;
    game_count(): number;
    game_pgn(n: number): string;
    constructor(si4: Uint8Array, sn4: Uint8Array, sg4: Uint8Array);
}

export function export_saved_game(json: string, format: string): string;

export function ironwoodSevenZipExtract(size: number, read: Function, write: Function, progress: Function): void;

export function ironwoodSevenZipList(size: number, read: Function): string;

export function prepare_archived_pgn(pgn: string): string;

export function preview_scoresheet(entries: string): string;

export function start(): Promise<void>;

/**
 * Checks scoresheet moves without changing the current game or saving anything.
 */
export function validate_scoresheet_pgn(pgn: string): number;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_scid5reader_free: (a: number, b: number) => void;
    readonly __wbg_scidimport_free: (a: number, b: number) => void;
    readonly export_saved_game: (a: number, b: number, c: number, d: number) => [number, number, number, number];
    readonly ironwoodSevenZipExtract: (a: number, b: any, c: any, d: any) => [number, number];
    readonly ironwoodSevenZipList: (a: number, b: any) => [number, number, number, number];
    readonly prepare_archived_pgn: (a: number, b: number) => [number, number, number, number];
    readonly preview_scoresheet: (a: number, b: number) => [number, number, number, number];
    readonly scid5reader_decode_game: (a: number, b: number, c: number, d: number, e: number) => [number, number, number, number];
    readonly scid5reader_index_entries: (a: number, b: number, c: number) => [number, number, number, number];
    readonly scid5reader_new: (a: number, b: number) => [number, number, number];
    readonly scidimport_game_count: (a: number) => number;
    readonly scidimport_game_pgn: (a: number, b: number) => [number, number, number, number];
    readonly scidimport_new: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly start: () => void;
    readonly validate_scoresheet_pgn: (a: number, b: number) => [number, number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h05b652a48fee2551: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h19d4157273093e84: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h19d4157273093e84_20: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h19d4157273093e84_21: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h0d9cf8010272419a: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h223c697a3bbe7068: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h223c697a3bbe7068_16: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h527fdb12970c4a14: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__h527fdb12970c4a14_19: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__hc86f24272c98cb0b: (a: number, b: number) => [number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
