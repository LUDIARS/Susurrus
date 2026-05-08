import { useEffect, useState } from "react";
import { api, type ChannelRow, type ForumRow, type ReplyRow, type ThreadRow } from "./api";

export function App() {
  const [forums, setForums] = useState<ForumRow[]>([]);
  const [channels, setChannels] = useState<ChannelRow[]>([]);
  const [threads, setThreads] = useState<ThreadRow[]>([]);
  const [replies, setReplies] = useState<ReplyRow[]>([]);
  const [selForum, setSelForum] = useState<ForumRow | null>(null);
  const [selChannel, setSelChannel] = useState<ChannelRow | null>(null);
  const [selThread, setSelThread] = useState<ThreadRow | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.listForums().then(setForums).catch(e => setError(String(e)));
  }, []);

  useEffect(() => {
    if (!selForum) { setChannels([]); return; }
    api.listChannels(selForum.id).then(setChannels).catch(e => setError(String(e)));
  }, [selForum?.id]);

  useEffect(() => {
    if (!selChannel) { setThreads([]); return; }
    api.listThreads(selChannel.id).then(setThreads).catch(e => setError(String(e)));
  }, [selChannel?.id]);

  useEffect(() => {
    if (!selThread) { setReplies([]); return; }
    api.listReplies(selThread.id).then(setReplies).catch(e => setError(String(e)));
  }, [selThread?.id]);

  return (
    <div className="app">
      {/* 左: forum + channel ツリー */}
      <div className="pane">
        <div className="pane-header">Forums</div>
        {forums.length === 0 ? (
          <div className="empty">forum がまだありません。 <br/>右下の compose で作成できます (未実装)。</div>
        ) : forums.map(f => (
          <div key={f.id}>
            <div
              className={`list-item ${selForum?.id === f.id ? "active" : ""}`}
              onClick={() => setSelForum(f)}
            >
              <div>{f.name}</div>
              <div className="muted">{f.path}</div>
            </div>
            {selForum?.id === f.id && channels.map(c => (
              <div
                key={c.id}
                className={`list-item ${selChannel?.id === c.id ? "active" : ""}`}
                style={{ paddingLeft: 24 }}
                onClick={() => setSelChannel(c)}
              >
                <div># {c.name}</div>
                {c.topic && <div className="muted">{c.topic}</div>}
              </div>
            ))}
          </div>
        ))}
      </div>

      {/* 中: thread 一覧 */}
      <div className="pane">
        <div className="pane-header">
          {selChannel ? `# ${selChannel.name}` : "(channel 未選択)"}
        </div>
        {threads.length === 0 ? (
          <div className="empty">スレッドがありません。</div>
        ) : threads.map(t => (
          <div
            key={t.id}
            className={`list-item ${selThread?.id === t.id ? "active" : ""}`}
            onClick={() => setSelThread(t)}
          >
            <div>{t.title}</div>
            <div className="muted">
              {t.reply_count} replies · {t.tags.map(x => `#${x}`).join(" ")}
            </div>
          </div>
        ))}
      </div>

      {/* 右: reply chain */}
      <div className="pane">
        <div className="pane-header">
          {selThread ? selThread.title : "(thread 未選択)"}
        </div>
        {error && <div className="empty" style={{ color: "#ff8888" }}>{error}</div>}
        {replies.length === 0 ? (
          <div className="empty">返信なし。</div>
        ) : replies.map(r => (
          <div key={r.id} className="reply">
            <div className="meta">{r.author} · {r.ts}</div>
            <div>{/* body は md 本文を再取得する別 API が必要 */}</div>
          </div>
        ))}
      </div>
    </div>
  );
}
