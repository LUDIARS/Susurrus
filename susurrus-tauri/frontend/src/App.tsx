import { useEffect, useState } from "react";
import {
  api,
  type ChannelRow,
  type ForumRow,
  type ReplyRow,
  type ThreadRow,
} from "./api";

// 暫定ユーザ URI。 Cernere 認証導線が入ったら置き換え。
const CURRENT_USER = "cr:local-user";

export function App() {
  const [forums, setForums] = useState<ForumRow[]>([]);
  const [channels, setChannels] = useState<ChannelRow[]>([]);
  const [threads, setThreads] = useState<ThreadRow[]>([]);
  const [replies, setReplies] = useState<ReplyRow[]>([]);
  const [bodies, setBodies] = useState<Record<string, string>>({});
  const [threadBody, setThreadBody] = useState<string>("");
  const [selForum, setSelForum] = useState<ForumRow | null>(null);
  const [selChannel, setSelChannel] = useState<ChannelRow | null>(null);
  const [selThread, setSelThread] = useState<ThreadRow | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [composerOpen, setComposerOpen] = useState(false);
  const [composerText, setComposerText] = useState("");

  const refreshForums = () => {
    api.listForums().then(setForums).catch(e => setError(String(e)));
  };

  useEffect(refreshForums, []);

  useEffect(() => {
    if (!selForum) { setChannels([]); return; }
    api.listChannels(selForum.id).then(cs => {
      setChannels(cs);
      if (cs.length > 0 && !selChannel) setSelChannel(cs[0]);
    }).catch(e => setError(String(e)));
  }, [selForum?.id]);

  useEffect(() => {
    if (!selChannel) { setThreads([]); return; }
    api.listThreads(selChannel.id).then(setThreads).catch(e => setError(String(e)));
  }, [selChannel?.id]);

  useEffect(() => {
    if (!selThread) { setReplies([]); setThreadBody(""); return; }
    api.readThreadBody(selThread.id).then(b => setThreadBody(b.body)).catch(e => setError(String(e)));
    api.listReplies(selThread.id).then(async (rs) => {
      setReplies(rs);
      // 各 reply の body を順次読み込む
      const bm: Record<string, string> = {};
      for (const r of rs) {
        try { bm[r.id] = (await api.readReplyBody(r.id)).body; } catch {}
      }
      setBodies(bm);
    }).catch(e => setError(String(e)));
  }, [selThread?.id]);

  const submitReply = async () => {
    if (!selForum || !selChannel || !selThread || !composerText.trim()) return;
    try {
      await api.createReply({
        forum_id: selForum.id,
        channel_id: selChannel.id,
        thread_id: selThread.id,
        thread_md_path: selThread.md_path,
        parent_id: selThread.id,
        body: composerText,
        author: CURRENT_USER,
        mentions: [],
      });
      setComposerText("");
      setComposerOpen(false);
      // refresh thread + replies
      const fresh = await api.listThreads(selChannel.id);
      setThreads(fresh);
      const updated = fresh.find(t => t.id === selThread.id);
      if (updated) setSelThread(updated);
      const rs = await api.listReplies(selThread.id);
      setReplies(rs);
      const bm: Record<string, string> = {};
      for (const r of rs) {
        try { bm[r.id] = (await api.readReplyBody(r.id)).body; } catch {}
      }
      setBodies(bm);
    } catch (e) { setError(String(e)); }
  };

  if (forums.length === 0) {
    return <SetupScreen onDone={refreshForums} setError={setError} />;
  }

  return (
    <div className="app">
      {/* 左: forum + channel ツリー */}
      <div className="pane">
        <div className="pane-header">Forums</div>
        {forums.map(f => (
          <div key={f.id}>
            <div
              className={`list-item ${selForum?.id === f.id ? "active" : ""}`}
              onClick={() => { setSelForum(f); setSelChannel(null); setSelThread(null); }}
            >
              <div>{f.name}</div>
              <div className="muted">{f.path}</div>
            </div>
            {selForum?.id === f.id && channels.map(c => (
              <div
                key={c.id}
                className={`list-item ${selChannel?.id === c.id ? "active" : ""}`}
                style={{ paddingLeft: 24 }}
                onClick={() => { setSelChannel(c); setSelThread(null); }}
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
        {selChannel && (
          <NewThreadButton
            forum={selForum!}
            channel={selChannel}
            onCreated={async (id) => {
              const fresh = await api.listThreads(selChannel.id);
              setThreads(fresh);
              const t = fresh.find(x => x.id === id);
              if (t) setSelThread(t);
            }}
            setError={setError}
          />
        )}
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
              {t.reply_count} replies
              {t.tags.length > 0 && " · " + t.tags.map(x => `#${x}`).join(" ")}
            </div>
          </div>
        ))}
      </div>

      {/* 右: reply chain + composer */}
      <div className="pane" style={{ display: "flex", flexDirection: "column" }}>
        <div className="pane-header">
          {selThread ? selThread.title : "(thread 未選択)"}
        </div>
        <div style={{ flex: 1, overflowY: "auto" }}>
          {error && <div className="empty" style={{ color: "#ff8888" }}>{error}</div>}
          {selThread && threadBody && (
            <div className="reply">
              <div className="meta">
                {selThread.author} · {selThread.ts} (thread root)
              </div>
              <div style={{ whiteSpace: "pre-wrap" }}>{threadBody}</div>
            </div>
          )}
          {replies.map(r => (
            <div key={r.id} className="reply">
              <div className="meta">{r.author} · {r.ts}</div>
              <div style={{ whiteSpace: "pre-wrap" }}>{bodies[r.id] ?? "…"}</div>
            </div>
          ))}
        </div>
        {selThread && (
          <div className="composer">
            {composerOpen ? (
              <>
                <textarea
                  value={composerText}
                  placeholder="返信を入力 (Markdown)"
                  onChange={e => setComposerText(e.target.value)}
                />
                <div>
                  <button onClick={submitReply}>送信</button>
                  <button
                    onClick={() => { setComposerOpen(false); setComposerText(""); }}
                    style={{ marginLeft: 6, background: "var(--bg-3)", color: "var(--fg)" }}
                  >
                    キャンセル
                  </button>
                </div>
              </>
            ) : (
              <button onClick={() => setComposerOpen(true)}>返信する</button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// ──────────────────────────────────────────────────────────────────
// 初回 setup: forum + channel が無いとき表示する。

function SetupScreen({
  onDone,
  setError,
}: {
  onDone: () => void;
  setError: (e: string) => void;
}) {
  const [forumPath, setForumPath] = useState("work/ludiars");
  const [forumName, setForumName] = useState("LUDIARS Workspace");
  const [channelName, setChannelName] = useState("general");
  const [busy, setBusy] = useState(false);
  const submit = async () => {
    setBusy(true);
    try {
      const forumId = await api.createForum({
        path: forumPath, name: forumName,
        visibility: "cernere-group",
        group: null,
        created_by: CURRENT_USER,
      });
      await api.createChannel({
        forum_id: forumId, forum_path: forumPath,
        name: channelName, topic: "",
        sort: 100,
        created_by: CURRENT_USER,
      });
      onDone();
    } catch (e) { setError(String(e)); }
    finally { setBusy(false); }
  };
  return (
    <div style={{ padding: 40, maxWidth: 480, margin: "0 auto" }}>
      <h2>Susurrus へようこそ</h2>
      <p className="muted">
        最初の forum + channel を作成します。 後から自由に追加 / 変更できます。
      </p>
      <label>forum path<br/>
        <input
          value={forumPath} onChange={e => setForumPath(e.target.value)}
          style={inputStyle}
        />
      </label>
      <label style={{ display: "block", marginTop: 12 }}>forum 名前<br/>
        <input
          value={forumName} onChange={e => setForumName(e.target.value)}
          style={inputStyle}
        />
      </label>
      <label style={{ display: "block", marginTop: 12 }}>最初の channel<br/>
        <input
          value={channelName} onChange={e => setChannelName(e.target.value)}
          style={inputStyle}
        />
      </label>
      <button
        onClick={submit} disabled={busy}
        style={{
          marginTop: 18,
          background: "var(--accent)", color: "#0d1116",
          border: "none", padding: "8px 18px", borderRadius: 4,
          fontWeight: 600, cursor: "pointer",
        }}
      >
        {busy ? "作成中…" : "作成する"}
      </button>
    </div>
  );
}

const inputStyle: React.CSSProperties = {
  width: "100%", boxSizing: "border-box",
  background: "var(--bg-3)", color: "var(--fg)",
  border: "1px solid var(--border)", borderRadius: 4,
  padding: "6px 8px", marginTop: 4, font: "inherit",
};

// ──────────────────────────────────────────────────────────────────

function NewThreadButton({
  forum, channel, onCreated, setError,
}: {
  forum: ForumRow;
  channel: ChannelRow;
  onCreated: (id: string) => void;
  setError: (e: string) => void;
}) {
  const [open, setOpen] = useState(false);
  const [title, setTitle] = useState("");
  const [body, setBody] = useState("");
  if (!open) {
    return (
      <div style={{ padding: 8 }}>
        <button
          onClick={() => setOpen(true)}
          style={{
            background: "var(--accent)", color: "#0d1116",
            border: "none", padding: "6px 12px", borderRadius: 4,
            fontWeight: 600, cursor: "pointer", width: "100%",
          }}
        >+ 新しいスレッド</button>
      </div>
    );
  }
  return (
    <div style={{ padding: 8, borderBottom: "1px solid var(--border)" }}>
      <input
        placeholder="タイトル"
        value={title} onChange={e => setTitle(e.target.value)}
        style={inputStyle}
      />
      <textarea
        placeholder="最初の投稿 (Markdown)"
        value={body} onChange={e => setBody(e.target.value)}
        style={{ ...inputStyle, minHeight: 60, marginTop: 6 }}
      />
      <div style={{ marginTop: 6 }}>
        <button
          onClick={async () => {
            try {
              const id = await api.createThread({
                forum_id: forum.id,
                channel_id: channel.id,
                channel_path: channel.path,
                title, body,
                tags: [],
                author: CURRENT_USER,
              });
              setTitle(""); setBody(""); setOpen(false);
              onCreated(id);
            } catch (e) { setError(String(e)); }
          }}
        >投稿</button>
        <button
          onClick={() => { setOpen(false); setTitle(""); setBody(""); }}
          style={{ marginLeft: 6, background: "var(--bg-3)", color: "var(--fg)" }}
        >キャンセル</button>
      </div>
    </div>
  );
}
