export interface FileItem {
  cid: string;
  name: string;
  is_dir: boolean;
  size: number;
  update_time: number;
  file_id?: string;
}

export interface MovieItem {
  file_id: string;
  title: string;
  original_title?: string | null;
  chinese_name?: string | null;
  year: number | null;
  poster_local: string | null;
  rating: number | null;
  genre: string[];
  is_hidden: boolean;
  progress?: number;
  duration?: number;
  group_names?: string[];
  is_favorite?: boolean;
  scrape_status?: number;
}

export interface MovieDetail extends MovieItem {
  original_title: string | null;
  backdrop_local: string | null;
  overview: string | null;
  runtime: number | null;
  director: string | null;
  actors: string[];
  file_name: string;
  file_size: number;
  created_at: number;
  updated_at: number;
  groups: GroupItem[];
}

export interface GroupItem {
  id: number;
  name: string;
  type: 'manual' | 'genre' | 'collection';
  sort_order: number;
  movie_count?: number;
}

export interface ActressItem {
  id: number;
  name: string;
  avatar_local: string | null;
  debut_year: number | null;
  height: number | null;
  bust: number | null;
  waist: number | null;
  hip: number | null;
  cup: string | null;
  letter: string;
  movie_count?: number;
  local_folder_name?: string | null;
  is_pending?: boolean;
  source?: string | null;
}

export interface MergeOptions {
  mergeFolders: boolean;
  conflictPolicy?: 'rename' | 'skip' | 'overwrite';
  dryRun?: boolean;
}

export interface MergeResult {
  success: boolean;
  movedFiles: string[];
  conflicts: string[];
  renamedFiles?: Array<{ from: string; to: string }>;
  error?: string;
}

export interface DuplicatePair {
  id1: number;
  id2: number;
  similarity: number;
}

export interface ActressGroupItem {
  id: number;
  name: string;
  sort_order: number;
  member_count?: number;
}

export interface FilterParams {
  keyword?: string;
  year?: number;
  genre?: string;
  group_id?: number;
  is_hidden?: boolean;
  is_finished?: boolean;
}

export interface Task {
  id: string;
  type: 'scan' | 'scrape';
  status: 'pending' | 'running' | 'paused' | 'completed' | 'failed';
  progress: number;
  result?: any;
  error?: string;
  created_at: number;
  updated_at: number;
}

export interface ScrapeResult {
  source: string;
  title: string;
  year?: number;
  poster_url?: string;
  backdrop_url?: string;
  overview?: string;
  rating?: number;
  runtime?: number;
  director?: string;
  genre?: string[];
  actors?: string[];
  score: number;
}
