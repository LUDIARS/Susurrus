// Tauri IPC への薄い wrapper。 commands.rs と 1:1 対応。
import { invoke } from "@tauri-apps/api/core";

export type Visibility = "public" | "cernere-group" | "invite-only" | "dm";

export interface ForumRow {
  id: string;
  path: string;
  name: string;
  parent_id: string | null;
  visibility: string;
}

export interface ChannelRow {
  id: string;
  forum_id: string;
  path: string;
  name: string;
  topic: string;
  sort: number;
  archived: boolean;
}

export interface ThreadRow {
  id: string;
  channel_id: string;
  forum_id: string;
  title: string;
  author: string;
  ts: string;
  last_reply_ts: string | null;
  reply_count: number;
  pinned: boolean;
  locked: boolean;
  tags: string[];
  md_path: string;
}

export interface BodyResponse {
  id: string;
  body: string;
  md_path: string;
}

export interface ReplyRow {
  id: string;
  thread_id: string;
  parent_id: string;
  author: string;
  ts: string;
  edited_at: string | null;
  mentions: string[];
}

export interface ReplySearchHit {
  reply_id: string;
  thread_id: string;
  author: string;
  ts: string;
  snippet: string;
}

export interface IndexStats {
  scanned: number;
  upserted: number;
  unchanged: number;
  failed: number;
}

export const api = {
  ping: () => invoke<string>("ping"),
  listForums: () => invoke<ForumRow[]>("list_forums"),
  listChannels: (forum_id: string) =>
    invoke<ChannelRow[]>("list_channels", { forumId: forum_id }),
  listThreads: (channel_id: string, limit = 50, offset = 0) =>
    invoke<ThreadRow[]>("list_threads", { channelId: channel_id, limit, offset }),
  listReplies: (thread_id: string) =>
    invoke<ReplyRow[]>("list_replies", { threadId: thread_id }),
  searchReplies: (q: string, limit = 50) =>
    invoke<ReplySearchHit[]>("search_replies", { q, limit }),
  reindexAll: () => invoke<IndexStats>("reindex_all"),
  readThreadBody: (thread_id: string) =>
    invoke<BodyResponse>("read_thread_body", { threadId: thread_id }),
  readReplyBody: (reply_id: string) =>
    invoke<BodyResponse>("read_reply_body", { replyId: reply_id }),
  createForum: (args: {
    path: string;
    name: string;
    visibility: Visibility;
    group: string | null;
    created_by: string;
  }) => invoke<string>("create_forum", { args }),
  createChannel: (args: {
    forum_id: string;
    forum_path: string;
    name: string;
    topic?: string;
    sort?: number;
    created_by: string;
  }) => invoke<string>("create_channel", { args }),
  createThread: (args: {
    forum_id: string;
    channel_id: string;
    channel_path: string;
    title: string;
    body: string;
    tags?: string[];
    author: string;
  }) => invoke<string>("create_thread", { args }),
  createReply: (args: {
    forum_id: string;
    channel_id: string;
    thread_id: string;
    thread_md_path: string;
    parent_id: string;
    body: string;
    author: string;
    mentions?: string[];
  }) => invoke<string>("create_reply", { args }),
};
