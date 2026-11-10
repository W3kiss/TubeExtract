import { invoke } from '@tauri-apps/api/core';
import type { SiteId } from './sites';

export type AppStatus = 'loading_components' | 'ready' | 'fetching_metadata' | 'downloading' | 'error';

export interface AppError {
	kind: 'network' | 'invalid_url' | 'unavailable' | 'format' | 'cancelled' | 'setup' | 'internal';
	message: string;
	details?: string | null;
}

export interface DepsStatus {
	ready: boolean;
	missing: string[];
	binDir: string;
}

export interface SetupProgress {
	component: string;
	label: string;
	index: number;
	count: number;
	stage: 'downloading' | 'extracting' | 'done';
	downloaded: number;
	total: number | null;
	overall: number;
}

export interface QualityOption {
	key: string;
	label: string;
	height: number;
	fps: number;
	approxSize: number | null;
}

export interface VideoInfo {
	id: string;
	title: string;
	uploader: string;
	thumbnail: string | null;
	duration: number | null;
	webpageUrl: string;
	extractor: string;
	hasVideo: boolean;
	qualities: QualityOption[];
	audioSize: number | null;
	subtitleLangs: string[];
	autoCaptionLangs: number;
}

export interface DownloadProgress {
	phase: string;
	percent: number | null;
	downloaded: number | null;
	total: number | null;
	speed: number | null;
	eta: number | null;
	stream: number;
}

export interface DownloadOptions {
	url: string;
	site: SiteId;
	mode: 'audio' | 'video';
	quality: { height: number; fps: number } | null;
	subtitles: boolean;
	subLangs: string;
	autoSubs: boolean;
	startTime: string | null;
	endTime: string | null;
	outputDir: string;
	title: string | null;
	artist: string | null;
	duration: number | null;
}

export interface DownloadResult {
	filePath: string;
	tagsWritten: boolean;
	coverEmbedded: boolean;
}

/** Normalises anything thrown by `invoke` into an AppError. */
export function toAppError(e: unknown): AppError {
	if (e && typeof e === 'object' && 'message' in e && 'kind' in e) return e as AppError;
	if (typeof e === 'string') return { kind: 'internal', message: e };
	return { kind: 'internal', message: e instanceof Error ? e.message : 'Unexpected error.' };
}

export const api = {
	checkDependencies: () => invoke<DepsStatus>('check_dependencies'),
	installDependencies: () => invoke<void>('install_dependencies'),
	fetchMetadata: (url: string, site: SiteId) => invoke<VideoInfo>('fetch_metadata', { url, site }),
	startDownload: (options: DownloadOptions) => invoke<DownloadResult>('start_download', { options }),
	cancelDownload: () => invoke<boolean>('cancel_download'),
	defaultDownloadDir: () => invoke<string | null>('default_download_dir'),
	ytdlpVersion: () => invoke<string>('ytdlp_version'),
	updateYtdlp: () => invoke<string>('update_ytdlp')
};
