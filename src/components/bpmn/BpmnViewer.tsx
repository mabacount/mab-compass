import React, { useEffect, useId, useMemo, useRef, useState } from "react";
import {
  CheckCircle2,
  ChevronLeft,
  Code2,
  Copy,
  Download,
  ExternalLink,
  FolderKanban,
  GitBranch,
  Layers,
  Maximize2,
  Play,
  Plus,
  User,
  Wrench,
  X,
  ZoomIn,
  ZoomOut,
} from "lucide-react";
import {
  BpmnGraph,
  PositionedBpmnNode,
  layoutBpmnGraph,
  parseBpmnXml,
} from "../../lib/bpmnEngine";
import { MethodPlaybook } from "../../lib/tauri_api";

interface BpmnViewerProps {
  bpmnXml: string;
  title?: string;
  allMethods?: MethodPlaybook[];
  onOpenMethod?: (method: MethodPlaybook) => void;
  onStartTask?: (taskId: string) => void;
  onQuickCompleteTask?: (taskId: string) => void;
  className?: string;
}

const NODE_KIND_LABELS: Record<string, string> = {
  startEvent: "رویداد شروع (Start Event)",
  endEvent: "رویداد پایان (End Event)",
  intermediateEvent: "رویداد میانی (Intermediate Event)",
  task: "گام کاری (Task)",
  userTask: "گام انسانی (User Task)",
  serviceTask: "گام سیستمی / خودکار (Service Task)",
  scriptTask: "اسکریپت (Script Task)",
  manualTask: "اقدام دستی (Manual Task)",
  subProcess: "مجموعه کار / زیرفرآیند (Sub-Process)",
  callActivity: "فراخوانی متد داخلی (Call Activity)",
  exclusiveGateway: "دروازه تصمیم XOR (Exclusive Gateway)",
  parallelGateway: "دروازه موازی AND (Parallel Gateway)",
  inclusiveGateway: "دروازه انتخابی OR (Inclusive Gateway)",
};

export function BpmnViewer({
  bpmnXml,
  title = "نمودار BPMN 2.0",
  allMethods = [],
  onOpenMethod,
  onStartTask,
  onQuickCompleteTask,
  className = "",
}: BpmnViewerProps) {
  const svgUid = useId().replace(/:/g, "");
  const arrowId = `bpmn-arrow-${svgUid}`;
  const arrowActiveId = `bpmn-arrow-active-${svgUid}`;

  const rootGraph = useMemo(() => parseBpmnXml(bpmnXml, title), [bpmnXml, title]);

  // Drill-down breadcrumb stack (starts at rootGraph)
  const [drillStack, setDrillStack] = useState<BpmnGraph[]>([rootGraph]);
  const [selectedNodeId, setSelectedNodeId] = useState<string | null>(null);
  const [showRawXml, setShowRawXml] = useState(false);
  const [copiedXml, setCopiedXml] = useState(false);

  // Pan & Zoom state
  const [zoom, setZoom] = useState(1);
  const [pan, setPan] = useState({ x: 0, y: 0 });
  const [isPanning, setIsPanning] = useState(false);
  const panStartRef = useRef<{ x: number; y: number; panX: number; panY: number } | null>(null);
  const svgContainerRef = useRef<HTMLDivElement | null>(null);

  // Preserve active drill-down SubProcess path when live dashboard updates refresh rootGraph
  useEffect(() => {
    setDrillStack((prev) => {
      if (prev.length <= 1 || prev[0]?.id !== rootGraph.id) {
        setSelectedNodeId(null);
        setZoom(1);
        setPan({ x: 0, y: 0 });
        return [rootGraph];
      }
      const rebuilt: BpmnGraph[] = [rootGraph];
      let current: BpmnGraph = rootGraph;
      for (let i = 1; i < prev.length; i++) {
        const targetId = prev[i].id;
        const matchNode = current.nodes.find((n) => n.subGraph?.id === targetId || n.id === targetId);
        if (matchNode?.subGraph && matchNode.subGraph.nodes.length > 0) {
          rebuilt.push(matchNode.subGraph);
          current = matchNode.subGraph;
        } else {
          break;
        }
      }
      return rebuilt;
    });
  }, [rootGraph]);

  const activeGraph = drillStack[drillStack.length - 1] || rootGraph;
  const positioned = useMemo(() => layoutBpmnGraph(activeGraph), [activeGraph]);

  const selectedNode: PositionedBpmnNode | null = useMemo(
    () => positioned.nodes.find((n) => n.id === selectedNodeId) ?? null,
    [positioned.nodes, selectedNodeId]
  );

  const methodById = useMemo(() => {
    const map = new Map<string, MethodPlaybook>();
    for (const m of allMethods) {
      map.set(m.id, m);
    }
    return map;
  }, [allMethods]);

  const handleDrillIntoNode = (node: PositionedBpmnNode) => {
    if (node.subGraph && node.subGraph.nodes.length > 0) {
      setDrillStack((prev) => [...prev, node.subGraph!]);
      setSelectedNodeId(null);
      setZoom(1);
      setPan({ x: 0, y: 0 });
      return;
    }
    if (node.subMethodId) {
      const linked = methodById.get(node.subMethodId);
      if (linked) {
        if (onOpenMethod) {
          onOpenMethod(linked);
        } else {
          const parsedChild = parseBpmnXml(linked.bpmn_xml, linked.title);
          if (parsedChild.nodes.length > 0) {
            setDrillStack((prev) => [...prev, parsedChild]);
            setSelectedNodeId(null);
            setZoom(1);
            setPan({ x: 0, y: 0 });
          }
        }
      }
    }
  };

  const handleFitView = () => {
    const el = svgContainerRef.current;
    if (!el || positioned.width <= 0 || positioned.height <= 0) {
      setZoom(1);
      setPan({ x: 0, y: 0 });
      return;
    }
    const rect = el.getBoundingClientRect();
    if (rect.width <= 40 || rect.height <= 40) return;
    const scaleX = (rect.width - 28) / positioned.width;
    const scaleY = (rect.height - 28) / positioned.height;
    const nextZoom = Math.max(0.45, Math.min(1.05, Math.min(scaleX, scaleY)));
    setZoom(Math.round(nextZoom * 100) / 100);
    setPan({ x: 0, y: 0 });
  };

  // Auto-fit diagram cleanly when switching graphs or drilling down so all nodes are visible immediately
  useEffect(() => {
    const id = window.requestAnimationFrame(() => {
      const el = svgContainerRef.current;
      if (!el || positioned.width <= 0 || positioned.height <= 0) return;
      const rect = el.getBoundingClientRect();
      if (rect.width <= 40 || rect.height <= 40) return;
      if (positioned.width > rect.width - 24 || positioned.height > rect.height - 24) {
        const scaleX = (rect.width - 28) / positioned.width;
        const scaleY = (rect.height - 28) / positioned.height;
        const nextZoom = Math.max(0.45, Math.min(1.0, Math.min(scaleX, scaleY)));
        setZoom(Math.round(nextZoom * 100) / 100);
        setPan({ x: 0, y: 0 });
      }
    });
    return () => window.cancelAnimationFrame(id);
  }, [activeGraph.id, positioned.width, positioned.height]);

  const handleCopyXml = async () => {
    try {
      await navigator.clipboard.writeText(bpmnXml);
      setCopiedXml(true);
      window.setTimeout(() => setCopiedXml(false), 2000);
    } catch {
      // ignore
    }
  };

  const handleDownloadXml = () => {
    const blob = new Blob([bpmnXml], { type: "application/xml;charset=utf-8" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${(title || "workflow").replace(/[^a-zA-Z0-9_\u0600-\u06FF-]/g, "_")}.bpmn`;
    a.click();
    URL.revokeObjectURL(url);
  };

  const canDrill = (node: PositionedBpmnNode): boolean =>
    Boolean(
      (node.subGraph && node.subGraph.nodes.length > 0) ||
        (node.subMethodId && methodById.has(node.subMethodId))
    );

  const getTaskStatusAccent = (status?: string) => {
    switch (status) {
      case "IN_PROGRESS":
        return { stroke: "#10B981", badgeBg: "#10B98126", badgeText: "#34D399", label: "در حال اجرا" };
      case "CLOSED":
        return { stroke: "#64748B", badgeBg: "#64748B26", badgeText: "#94A3B8", label: "تکمیل‌شده" };
      case "BLOCKED":
        return { stroke: "#F43F5E", badgeBg: "#F43F5E26", badgeText: "#FB7185", label: "مسدود" };
      case "QUEUED_IN_CHAIN":
        return { stroke: "#F59E0B", badgeBg: "#F59E0B26", badgeText: "#FBBF24", label: "در صف زنجیره" };
      default:
        return { stroke: "#3B82F6", badgeBg: "#3B82F626", badgeText: "#60A5FA", label: "باز" };
    }
  };

  return (
    <div
      dir="rtl"
      className={`flex flex-col h-full min-h-0 bg-[#0B0C0F] border border-white/[0.07] rounded-xl overflow-hidden ${className}`}
    >
      {/* Top Toolbar + Breadcrumbs */}
      <div className="px-3 py-2 bg-[#121317] border-b border-white/[0.07] flex items-center justify-between gap-2 flex-wrap shrink-0">
        {/* Breadcrumb navigation */}
        <div className="flex items-center gap-1 min-w-0 flex-wrap text-[11.5px]">
          <span className="px-1.5 py-0.5 rounded bg-purple-500/15 border border-purple-500/30 text-purple-300 font-mono text-[10px] shrink-0">
            BPMN 2.0
          </span>
          {drillStack.map((g, idx) => {
            const isLast = idx === drillStack.length - 1;
            return (
              <React.Fragment key={`${g.id}-${idx}`}>
                {idx > 0 && <ChevronLeft className="w-3.5 h-3.5 text-white/30 shrink-0" />}
                <button
                  type="button"
                  onClick={() => {
                    if (!isLast) {
                      setDrillStack((prev) => prev.slice(0, idx + 1));
                      setSelectedNodeId(null);
                      setZoom(1);
                      setPan({ x: 0, y: 0 });
                    }
                  }}
                  className={`px-2 py-0.5 rounded-md transition truncate max-w-[200px] ${
                    isLast
                      ? "bg-white/[0.08] text-white font-semibold"
                      : "text-mab-text-secondary hover:text-white hover:bg-white/[0.05]"
                  }`}
                  title={g.name}
                >
                  {g.name}
                </button>
              </React.Fragment>
            );
          })}
        </div>

        {/* Zoom & XML Controls */}
        <div className="flex items-center gap-1 shrink-0">
          {!showRawXml && (
            <>
              <button
                type="button"
                onClick={() => setZoom((z) => Math.max(0.4, Math.round((z - 0.15) * 100) / 100))}
                title="کوچک‌نمایی"
                className="p-1.5 rounded-md bg-white/[0.03] hover:bg-white/[0.08] text-white/70 hover:text-white transition"
              >
                <ZoomOut className="w-3.5 h-3.5" />
              </button>
              <span className="font-mono text-[10.5px] text-white/60 w-10 text-center">
                {Math.round(zoom * 100)}%
              </span>
              <button
                type="button"
                onClick={() => setZoom((z) => Math.min(2.0, Math.round((z + 0.15) * 100) / 100))}
                title="بزرگ‌نمایی"
                className="p-1.5 rounded-md bg-white/[0.03] hover:bg-white/[0.08] text-white/70 hover:text-white transition"
              >
                <ZoomIn className="w-3.5 h-3.5" />
              </button>
              <button
                type="button"
                onClick={handleFitView}
                title="تنظیم خودکار کادر"
                className="p-1.5 rounded-md bg-white/[0.03] hover:bg-white/[0.08] text-white/70 hover:text-white transition"
              >
                <Maximize2 className="w-3.5 h-3.5" />
              </button>
              <div className="w-[1px] h-4 bg-white/[0.08] mx-0.5" />
            </>
          )}

          <button
            type="button"
            onClick={() => setShowRawXml((v) => !v)}
            className={`px-2 py-1 rounded-md text-[11px] font-medium inline-flex items-center gap-1 transition ${
              showRawXml
                ? "bg-blue-500/20 border border-blue-500/35 text-blue-300"
                : "bg-white/[0.03] hover:bg-white/[0.08] border border-white/[0.07] text-white/70 hover:text-white"
            }`}
            title="مشاهده کد استاندارد BPMN 2.0 XML"
          >
            <Code2 className="w-3.5 h-3.5" />
            <span>{showRawXml ? "نمای گرافیکی" : "کد XML"}</span>
          </button>

          <button
            type="button"
            onClick={() => void handleCopyXml()}
            title="کپی XML"
            className="p-1.5 rounded-md bg-white/[0.03] hover:bg-white/[0.08] text-white/70 hover:text-white transition"
          >
            {copiedXml ? (
              <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
            ) : (
              <Copy className="w-3.5 h-3.5" />
            )}
          </button>

          <button
            type="button"
            onClick={handleDownloadXml}
            title="دانلود فایل .bpmn"
            className="p-1.5 rounded-md bg-white/[0.03] hover:bg-white/[0.08] text-white/70 hover:text-white transition"
          >
            <Download className="w-3.5 h-3.5" />
          </button>
        </div>
      </div>

      {/* Main Canvas Area */}
      {showRawXml ? (
        <div className="flex-1 min-h-0 overflow-auto p-4 bg-[#0A0B0E]" dir="ltr">
          <pre className="font-mono text-[11.5px] leading-relaxed text-slate-300 whitespace-pre select-text">
            {bpmnXml}
          </pre>
        </div>
      ) : (
        <div className="flex-1 min-h-0 flex flex-col relative overflow-hidden">
          {/* Interactive SVG Canvas */}
          <div
            ref={svgContainerRef}
            onMouseDown={(e) => {
              if ((e.target as HTMLElement).closest("[data-bpmn-node='true']")) return;
              setIsPanning(true);
              panStartRef.current = {
                x: e.clientX,
                y: e.clientY,
                panX: pan.x,
                panY: pan.y,
              };
            }}
            onMouseMove={(e) => {
              if (!isPanning || !panStartRef.current) return;
              const dx = e.clientX - panStartRef.current.x;
              const dy = e.clientY - panStartRef.current.y;
              setPan({
                x: panStartRef.current.panX + dx,
                y: panStartRef.current.panY + dy,
              });
            }}
            onMouseUp={() => {
              setIsPanning(false);
              panStartRef.current = null;
            }}
            onMouseLeave={() => {
              setIsPanning(false);
              panStartRef.current = null;
            }}
            onWheel={(e) => {
              if (e.ctrlKey) {
                e.preventDefault();
                const delta = e.deltaY < 0 ? 0.1 : -0.1;
                setZoom((z) => Math.max(0.4, Math.min(2.0, Math.round((z + delta) * 100) / 100)));
              }
            }}
            className={`flex-1 min-h-[260px] overflow-auto relative select-none ${
              isPanning ? "cursor-grabbing" : "cursor-grab"
            }`}
            style={{
              backgroundImage:
                "radial-gradient(rgba(255,255,255,0.06) 1px, transparent 1px)",
              backgroundSize: "20px 20px",
            }}
            dir="ltr"
          >
            {positioned.nodes.length === 0 ? (
              <div
                dir="rtl"
                className="h-full flex flex-col items-center justify-center text-center p-6 text-mab-text-muted space-y-2"
              >
                <Layers className="w-8 h-8 text-white/20" />
                <p className="text-[13px] text-white/70 font-medium">
                  عنصری در این نمودار BPMN یافت نشد.
                </p>
                {activeGraph.documentation && (
                  <p className="text-[11.5px] max-w-md">{activeGraph.documentation}</p>
                )}
              </div>
            ) : (
              <svg
                width={Math.max(positioned.width * zoom + Math.max(0, pan.x) + 120, 680)}
                height={Math.max(positioned.height * zoom + Math.max(0, pan.y) + 90, 340)}
                className="block"
              >
                <defs>
                  <marker
                    id={arrowId}
                    viewBox="0 0 10 10"
                    refX="9"
                    refY="5"
                    markerWidth="6.5"
                    markerHeight="6.5"
                    orient="auto-start-reverse"
                  >
                    <path d="M 0 1 L 10 5 L 0 9 z" fill="#64748B" />
                  </marker>
                  <marker
                    id={arrowActiveId}
                    viewBox="0 0 10 10"
                    refX="9"
                    refY="5"
                    markerWidth="6.5"
                    markerHeight="6.5"
                    orient="auto-start-reverse"
                  >
                    <path d="M 0 1 L 10 5 L 0 9 z" fill="#3B82F6" />
                  </marker>
                </defs>

                <g transform={`translate(${pan.x}, ${pan.y}) scale(${zoom})`}>
                  {/* Edges */}
                  {positioned.edges.map((edge) => {
                    const isHighlighted =
                      selectedNodeId === edge.sourceRef || selectedNodeId === edge.targetRef;
                    return (
                      <g key={edge.id}>
                        <path
                          d={edge.path}
                          fill="none"
                          stroke={isHighlighted ? "#3B82F6" : "#475569"}
                          strokeWidth={isHighlighted ? 2.2 : 1.6}
                          markerEnd={
                            isHighlighted ? `url(#${arrowActiveId})` : `url(#${arrowId})`
                          }
                        />
                        {edge.name && (
                          <g transform={`translate(${edge.labelX}, ${edge.labelY})`}>
                            <rect
                              x={-44}
                              y={-10}
                              width={88}
                              height={20}
                              rx={6}
                              fill="#16171C"
                              stroke="rgba(255,255,255,0.12)"
                            />
                            <text
                              textAnchor="middle"
                              dominantBaseline="central"
                              fill="#CBD5E1"
                              fontSize="10"
                              fontFamily="Vazirmatn, sans-serif"
                            >
                              {edge.name.slice(0, 16)}
                            </text>
                          </g>
                        )}
                      </g>
                    );
                  })}

                  {/* Nodes */}
                  {positioned.nodes.map((node) => {
                    const isSelected = node.id === selectedNodeId;
                    const drillable = canDrill(node);
                    const cx = node.x + node.width / 2;
                    const cy = node.y + node.height / 2;

                    // Start / End / Intermediate Events
                    if (
                      node.kind === "startEvent" ||
                      node.kind === "endEvent" ||
                      node.kind === "intermediateEvent"
                    ) {
                      const stroke =
                        node.kind === "startEvent"
                          ? "#10B981"
                          : node.kind === "endEvent"
                          ? "#F43F5E"
                          : "#F59E0B";
                      return (
                        <g
                          key={node.id}
                          data-bpmn-node="true"
                          className="cursor-pointer"
                          onClick={() => setSelectedNodeId(node.id)}
                        >
                          <circle
                            cx={cx}
                            cy={cy}
                            r={21}
                            fill={isSelected ? `${stroke}33` : "#16171C"}
                            stroke={stroke}
                            strokeWidth={node.kind === "endEvent" ? 3.5 : 2}
                          />
                          {node.kind === "intermediateEvent" && (
                            <circle
                              cx={cx}
                              cy={cy}
                              r={16}
                              fill="none"
                              stroke={stroke}
                              strokeWidth={1.3}
                            />
                          )}
                          <text
                            x={cx}
                            y={node.y + node.height + 16}
                            textAnchor="middle"
                            fill="#E2E8F0"
                            fontSize="11"
                            fontWeight="600"
                            fontFamily="Vazirmatn, sans-serif"
                          >
                            {node.name}
                          </text>
                        </g>
                      );
                    }

                    // Gateways (Diamonds)
                    if (
                      node.kind === "exclusiveGateway" ||
                      node.kind === "parallelGateway" ||
                      node.kind === "inclusiveGateway"
                    ) {
                      const r = node.width / 2;
                      const points = `${cx},${cy - r} ${cx + r},${cy} ${cx},${cy + r} ${cx - r},${cy}`;
                      const symbol =
                        node.kind === "parallelGateway"
                          ? "+"
                          : node.kind === "inclusiveGateway"
                          ? "○"
                          : "×";
                      return (
                        <g
                          key={node.id}
                          data-bpmn-node="true"
                          className="cursor-pointer"
                          onClick={() => setSelectedNodeId(node.id)}
                        >
                          <polygon
                            points={points}
                            fill={isSelected ? "#F59E0B2E" : "#18191E"}
                            stroke={isSelected ? "#FBBF24" : "#F59E0B"}
                            strokeWidth={isSelected ? 2.4 : 1.8}
                          />
                          <text
                            x={cx}
                            y={cy + 1}
                            textAnchor="middle"
                            dominantBaseline="central"
                            fill="#FCD34D"
                            fontSize="20"
                            fontWeight="700"
                          >
                            {symbol}
                          </text>
                          <text
                            x={cx}
                            y={node.y + node.height + 16}
                            textAnchor="middle"
                            fill="#CBD5E1"
                            fontSize="10.5"
                            fontWeight="500"
                            fontFamily="Vazirmatn, sans-serif"
                          >
                            {node.name.length > 24 ? `${node.name.slice(0, 23)}…` : node.name}
                          </text>
                        </g>
                      );
                    }

                    // SubProcess / CallActivity / Tasks
                    const isSub = node.kind === "subProcess" || node.kind === "callActivity";
                    const statusStyle = node.taskStatus
                      ? getTaskStatusAccent(node.taskStatus)
                      : null;
                    const borderColor = isSelected
                      ? "#3B82F6"
                      : isSub
                      ? "#8B5CF6"
                      : statusStyle
                      ? statusStyle.stroke
                      : "rgba(255,255,255,0.16)";
                    const fillColor = isSelected
                      ? "#1D2436"
                      : isSub
                      ? "#1A1728"
                      : "#16171C";
                    const subCount = node.subGraph?.nodes.filter(
                      (sn) => sn.kind !== "startEvent" && sn.kind !== "endEvent"
                    ).length;

                    return (
                      <g
                        key={node.id}
                        data-bpmn-node="true"
                        className="cursor-pointer"
                        onClick={() => setSelectedNodeId(node.id)}
                        onDoubleClick={() => {
                          if (drillable) handleDrillIntoNode(node);
                        }}
                      >
                        <rect
                          x={node.x}
                          y={node.y}
                          width={node.width}
                          height={node.height}
                          rx={12}
                          fill={fillColor}
                          stroke={borderColor}
                          strokeWidth={isSelected ? 2.4 : isSub ? 1.8 : 1.3}
                          strokeDasharray={node.kind === "callActivity" ? "5 3" : undefined}
                        />

                        {/* Left accent bar */}
                        <rect
                          x={node.x + node.width - 4}
                          y={node.y + 8}
                          width={4}
                          height={node.height - 16}
                          rx={2}
                          fill={isSub ? "#8B5CF6" : statusStyle?.stroke || "#3B82F6"}
                        />

                        {/* Inner HTML foreignObject for crisp Persian RTL text wrapping */}
                        <foreignObject
                          x={node.x + 8}
                          y={node.y + 6}
                          width={node.width - 20}
                          height={node.height - (drillable ? 24 : 12)}
                        >
                          <div
                            dir="rtl"
                            className="h-full flex flex-col justify-between text-right overflow-hidden"
                          >
                            <div className="flex items-center justify-between gap-1 text-[9.5px] text-white/55">
                              <span className="truncate font-mono">
                                {isSub
                                  ? "SubProcess"
                                  : node.kind === "serviceTask"
                                  ? "Service"
                                  : node.kind === "userTask"
                                  ? "UserTask"
                                  : "Task"}
                              </span>
                              {statusStyle ? (
                                <span
                                  className="px-1.5 py-[2px] rounded text-[9px] font-semibold shrink-0"
                                  style={{
                                    backgroundColor: statusStyle.badgeBg,
                                    color: statusStyle.badgeText,
                                  }}
                                >
                                  {statusStyle.label}
                                </span>
                              ) : subCount !== undefined && subCount > 0 ? (
                                <span className="px-1.5 py-[2px] rounded bg-purple-500/20 text-purple-300 text-[9px] font-semibold shrink-0">
                                  {subCount} گام داخلی
                                </span>
                              ) : null}
                            </div>

                            <div className="text-[11.5px] font-semibold text-[#F3F4F6] leading-snug line-clamp-2">
                              {node.name}
                            </div>

                            {node.laneName && (
                              <div className="text-[9.5px] text-blue-300/80 truncate">
                                {node.laneName}
                              </div>
                            )}
                          </div>
                        </foreignObject>

                        {/* BPMN [+] SubProcess Drill-Down Marker */}
                        {drillable && (
                          <g
                            transform={`translate(${cx - 10}, ${node.y + node.height - 18})`}
                            onClick={(e) => {
                              e.stopPropagation();
                              handleDrillIntoNode(node);
                            }}
                          >
                            <rect
                              width={20}
                              height={15}
                              rx={4}
                              fill="#8B5CF6"
                              fillOpacity={0.25}
                              stroke="#A78BFA"
                              strokeWidth={1.1}
                            />
                            <text
                              x={10}
                              y={8}
                              textAnchor="middle"
                              dominantBaseline="central"
                              fill="#DDD6FE"
                              fontSize="12"
                              fontWeight="700"
                            >
                              +
                            </text>
                          </g>
                        )}
                      </g>
                    );
                  })}
                </g>
              </svg>
            )}

            {/* Floating helper tip */}
            <div
              dir="rtl"
              className="absolute bottom-2.5 right-3 px-2.5 py-1 rounded-lg bg-[#121317]/90 border border-white/[0.08] text-[10.5px] text-mab-text-muted pointer-events-none"
            >
              کلیک روی هر بخش: مشاهده جزئیات • دوبارکلیک روی مجموعه کارهای بنفش (+): ورود به ورک‌فلوی داخلی
            </div>
          </div>

          {/* Clickable Element Detail Inspector Drawer */}
          {selectedNode && (
            <div
              dir="rtl"
              className="border-t border-white/[0.09] bg-[#131418] p-3.5 space-y-2.5 max-h-[46%] overflow-y-auto shrink-0 select-text"
            >
              <div className="flex items-start justify-between gap-2">
                <div className="space-y-1 min-w-0">
                  <div className="flex items-center gap-1.5 flex-wrap">
                    <span className="px-2 py-0.5 rounded bg-blue-500/15 border border-blue-500/30 text-blue-300 text-[10.5px] font-semibold inline-flex items-center gap-1">
                      {selectedNode.kind === "subProcess" ? (
                        <FolderKanban className="w-3 h-3" />
                      ) : selectedNode.kind === "serviceTask" ? (
                        <Wrench className="w-3 h-3" />
                      ) : selectedNode.kind === "userTask" ? (
                        <User className="w-3 h-3" />
                      ) : (
                        <GitBranch className="w-3 h-3" />
                      )}
                      <span>{NODE_KIND_LABELS[selectedNode.kind] || selectedNode.kind}</span>
                    </span>
                    <span className="font-mono text-[10px] text-white/45">{selectedNode.id}</span>
                    {selectedNode.role && (
                      <span className="px-2 py-0.5 rounded bg-white/[0.05] text-white/75 text-[10.5px]">
                        مسئول: {selectedNode.role}
                      </span>
                    )}
                  </div>
                  <h4 className="text-[14px] font-bold text-white leading-snug">
                    {selectedNode.name}
                  </h4>
                </div>

                <button
                  type="button"
                  onClick={() => setSelectedNodeId(null)}
                  className="p-1 rounded-md bg-white/[0.04] hover:bg-white/[0.09] text-white/60 hover:text-white transition shrink-0"
                  title="بستن جزئیات"
                >
                  <X className="w-3.5 h-3.5" />
                </button>
              </div>

              {selectedNode.documentation && (
                <p className="text-[12.5px] text-mab-text-secondary whitespace-pre-wrap leading-relaxed bg-[#0E0F12] border border-white/[0.06] rounded-lg p-2.5">
                  {selectedNode.documentation}
                </p>
              )}

              {((selectedNode.inputs && selectedNode.inputs.length > 0) ||
                (selectedNode.outputs && selectedNode.outputs.length > 0)) && (
                <div className="grid grid-cols-1 sm:grid-cols-2 gap-2 text-[11.5px]">
                  {selectedNode.inputs && selectedNode.inputs.length > 0 && (
                    <div className="p-2 rounded-lg bg-white/[0.02] border border-white/[0.06]">
                      <span className="text-blue-400 font-semibold block mb-1">ورودی‌ها:</span>
                      <div className="flex flex-wrap gap-1">
                        {selectedNode.inputs.map((inp, i) => (
                          <span
                            key={i}
                            className="px-2 py-0.5 rounded bg-white/[0.05] text-white/80"
                          >
                            {inp}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}
                  {selectedNode.outputs && selectedNode.outputs.length > 0 && (
                    <div className="p-2 rounded-lg bg-white/[0.02] border border-white/[0.06]">
                      <span className="text-emerald-400 font-semibold block mb-1">خروجی‌ها:</span>
                      <div className="flex flex-wrap gap-1">
                        {selectedNode.outputs.map((out, i) => (
                          <span
                            key={i}
                            className="px-2 py-0.5 rounded bg-white/[0.05] text-white/80"
                          >
                            {out}
                          </span>
                        ))}
                      </div>
                    </div>
                  )}
                </div>
              )}

              {selectedNode.checklist && selectedNode.checklist.length > 0 && (
                <div className="p-2.5 rounded-lg bg-emerald-500/[0.05] border border-emerald-500/20 space-y-1">
                  <span className="text-[11px] font-bold text-emerald-400 block">
                    چک‌لیست کنترل این مرحله:
                  </span>
                  <ul className="space-y-1 text-[12px] text-white/85">
                    {selectedNode.checklist.map((c, idx) => (
                      <li key={idx} className="flex items-start gap-1.5">
                        <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400 shrink-0 mt-0.5" />
                        <span>{c}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              )}

              {selectedNode.notes && (
                <div className="p-2.5 rounded-lg bg-amber-500/[0.06] border border-amber-500/20 text-[12px] text-amber-200/90 whitespace-pre-wrap">
                  {selectedNode.notes}
                </div>
              )}

              {/* Contextual Action Buttons */}
              <div className="flex items-center gap-2 flex-wrap pt-1">
                {canDrill(selectedNode) && (
                  <button
                    type="button"
                    onClick={() => handleDrillIntoNode(selectedNode)}
                    className="px-3 py-1.5 rounded-lg bg-purple-600 hover:bg-purple-500 text-white text-[12px] font-semibold inline-flex items-center gap-1.5 transition shadow-sm"
                  >
                    <Plus className="w-3.5 h-3.5" />
                    <span>ورود به ورک‌فلوی داخلی این مجموعه کار</span>
                    <ExternalLink className="w-3 h-3" />
                  </button>
                )}

                {selectedNode.taskId &&
                  selectedNode.taskStatus === "OPEN" &&
                  onStartTask && (
                    <button
                      type="button"
                      onClick={() => onStartTask(selectedNode.taskId!)}
                      className="px-3 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-[12px] font-semibold inline-flex items-center gap-1.5 transition"
                    >
                      <Play className="w-3.5 h-3.5" />
                      <span>شروع گام [{selectedNode.taskId}]</span>
                    </button>
                  )}

                {selectedNode.taskId &&
                  selectedNode.taskStatus !== "CLOSED" &&
                  selectedNode.taskStatus !== "QUEUED_IN_CHAIN" &&
                  onQuickCompleteTask && (
                    <button
                      type="button"
                      onClick={() => onQuickCompleteTask(selectedNode.taskId!)}
                      className="px-3 py-1.5 rounded-lg bg-emerald-600 hover:bg-emerald-500 text-white text-[12px] font-semibold inline-flex items-center gap-1.5 transition"
                    >
                      <CheckCircle2 className="w-3.5 h-3.5" />
                      <span>تکمیل گام [{selectedNode.taskId}]</span>
                    </button>
                  )}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default BpmnViewer;
