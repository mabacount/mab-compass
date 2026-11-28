import { DashboardState, WorkEntity, WorkLine } from "./tauri_api";

export type BpmnNodeKind =
  | "startEvent"
  | "endEvent"
  | "intermediateEvent"
  | "task"
  | "userTask"
  | "serviceTask"
  | "scriptTask"
  | "manualTask"
  | "subProcess"
  | "callActivity"
  | "exclusiveGateway"
  | "parallelGateway"
  | "inclusiveGateway";

export interface BpmnNode {
  id: string;
  name: string;
  kind: BpmnNodeKind;
  documentation: string;
  laneName?: string;
  calledElement?: string;
  subMethodId?: string;
  taskId?: string; // If linked to a real Mab-Compass WorkEntity task
  taskStatus?: string;
  workLineId?: string;
  role?: string;
  inputs?: string[];
  outputs?: string[];
  checklist?: string[];
  notes?: string;
  subGraph?: BpmnGraph; // Nested BPMN 2.0 workflow inside <bpmn:subProcess>
}

export interface BpmnEdge {
  id: string;
  sourceRef: string;
  targetRef: string;
  name?: string;
}

export interface BpmnGraph {
  id: string;
  name: string;
  documentation: string;
  nodes: BpmnNode[];
  edges: BpmnEdge[];
}

export interface PositionedBpmnNode extends BpmnNode {
  x: number;
  y: number;
  width: number;
  height: number;
  rank: number;
  row: number;
}

export interface PositionedBpmnEdge extends BpmnEdge {
  path: string;
  labelX: number;
  labelY: number;
}

export interface PositionedBpmnGraph {
  id: string;
  name: string;
  documentation: string;
  nodes: PositionedBpmnNode[];
  edges: PositionedBpmnEdge[];
  width: number;
  height: number;
}

function localTagName(el: Element): string {
  return el.localName || el.tagName.split(":").pop() || "";
}

function getDirectChildrenByLocalName(parent: Element, localName: string): Element[] {
  const out: Element[] = [];
  for (let i = 0; i < parent.children.length; i++) {
    const child = parent.children[i];
    if (localTagName(child) === localName) {
      out.push(child);
    }
  }
  return out;
}

function extractDocumentation(el: Element): string {
  const docs = getDirectChildrenByLocalName(el, "documentation");
  return docs
    .map((d) => (d.textContent || "").trim())
    .filter(Boolean)
    .join("\n");
}

function parseDocMetadata(doc: string): {
  cleanDescription: string;
  role?: string;
  inputs: string[];
  outputs: string[];
  checklist: string[];
  notes?: string;
  subMethodId?: string;
  taskId?: string;
  taskStatus?: string;
  workLineId?: string;
} {
  const lines = doc.split(/\r?\n/);
  const descLines: string[] = [];
  const inputs: string[] = [];
  const outputs: string[] = [];
  const checklist: string[] = [];
  let role: string | undefined;
  let notes: string | undefined;
  let subMethodId: string | undefined;
  let taskId: string | undefined;
  let taskStatus: string | undefined;
  let workLineId: string | undefined;
  let inChecklist = false;

  for (const raw of lines) {
    const line = raw.trim();
    if (!line) {
      inChecklist = false;
      continue;
    }
    if (line.startsWith("SUB_METHOD_ID:")) {
      subMethodId = line.slice("SUB_METHOD_ID:".length).trim();
      continue;
    }
    if (line.startsWith("TASK_ID:")) {
      taskId = line.slice("TASK_ID:".length).trim();
      continue;
    }
    if (line.startsWith("TASK_STATUS:")) {
      taskStatus = line.slice("TASK_STATUS:".length).trim();
      continue;
    }
    if (line.startsWith("WORK_LINE_ID:")) {
      workLineId = line.slice("WORK_LINE_ID:".length).trim();
      continue;
    }
    if (line.startsWith("نقش/مسئول:") || line.startsWith("نقش:")) {
      role = line.split(":").slice(1).join(":").trim();
      inChecklist = false;
      continue;
    }
    if (line.startsWith("ورودی‌ها:")) {
      const rest = line.slice("ورودی‌ها:".length).trim();
      inputs.push(...rest.split(/[،,]/).map((s) => s.trim()).filter(Boolean));
      inChecklist = false;
      continue;
    }
    if (line.startsWith("خروجی‌ها:")) {
      const rest = line.slice("خروجی‌ها:".length).trim();
      outputs.push(...rest.split(/[،,]/).map((s) => s.trim()).filter(Boolean));
      inChecklist = false;
      continue;
    }
    if (line.startsWith("چک‌لیست:")) {
      inChecklist = true;
      const rest = line.slice("چک‌لیست:".length).trim();
      if (rest) checklist.push(rest);
      continue;
    }
    if (line.startsWith("نکات:") || line.startsWith("اقدام اول:")) {
      notes = notes ? `${notes}\n${line}` : line;
      inChecklist = false;
      continue;
    }
    if (inChecklist && (line.startsWith("- ") || line.startsWith("* "))) {
      checklist.push(line.slice(2).trim());
      continue;
    }
    descLines.push(line);
  }

  return {
    cleanDescription: descLines.join("\n"),
    role,
    inputs,
    outputs,
    checklist,
    notes,
    subMethodId,
    taskId,
    taskStatus,
    workLineId,
  };
}

function mapTagToKind(tag: string): BpmnNodeKind | null {
  switch (tag) {
    case "startEvent":
      return "startEvent";
    case "endEvent":
      return "endEvent";
    case "intermediateCatchEvent":
    case "intermediateThrowEvent":
    case "boundaryEvent":
      return "intermediateEvent";
    case "task":
      return "task";
    case "userTask":
      return "userTask";
    case "serviceTask":
      return "serviceTask";
    case "scriptTask":
      return "scriptTask";
    case "manualTask":
    case "businessRuleTask":
    case "receiveTask":
    case "sendTask":
      return "manualTask";
    case "subProcess":
    case "adHocSubProcess":
    case "transaction":
      return "subProcess";
    case "callActivity":
      return "callActivity";
    case "exclusiveGateway":
    case "eventBasedGateway":
      return "exclusiveGateway";
    case "parallelGateway":
      return "parallelGateway";
    case "inclusiveGateway":
    case "complexGateway":
      return "inclusiveGateway";
    default:
      return null;
  }
}

function parseContainerElement(container: Element, fallbackName: string): BpmnGraph {
  const id = container.getAttribute("id") || "Process_1";
  const name = container.getAttribute("name") || fallbackName;
  const documentation = extractDocumentation(container);

  // Extract optional Lane assignments
  const laneByNodeId = new Map<string, string>();
  const laneSets = getDirectChildrenByLocalName(container, "laneSet");
  for (const ls of laneSets) {
    const lanes = getDirectChildrenByLocalName(ls, "lane");
    for (const lane of lanes) {
      const laneName = lane.getAttribute("name") || lane.getAttribute("id") || "";
      const refs = getDirectChildrenByLocalName(lane, "flowNodeRef");
      for (const r of refs) {
        const refId = (r.textContent || "").trim();
        if (refId && laneName) {
          laneByNodeId.set(refId, laneName);
        }
      }
    }
  }

  const nodes: BpmnNode[] = [];
  const edges: BpmnEdge[] = [];

  for (let i = 0; i < container.children.length; i++) {
    const child = container.children[i];
    const tag = localTagName(child);
    if (tag === "sequenceFlow") {
      const fid = child.getAttribute("id") || `Flow_${i}`;
      const sourceRef = child.getAttribute("sourceRef") || "";
      const targetRef = child.getAttribute("targetRef") || "";
      const flowName = child.getAttribute("name") || undefined;
      if (sourceRef && targetRef) {
        edges.push({ id: fid, sourceRef, targetRef, name: flowName });
      }
      continue;
    }

    const kind = mapTagToKind(tag);
    if (!kind) continue;

    const nid = child.getAttribute("id") || `${tag}_${i}`;
    const nname =
      child.getAttribute("name") ||
      (kind === "startEvent" ? "شروع" : kind === "endEvent" ? "پایان" : nid);
    const rawDoc = extractDocumentation(child);
    const meta = parseDocMetadata(rawDoc);
    const calledElement = child.getAttribute("calledElement") || undefined;

    let subGraph: BpmnGraph | undefined;
    if (kind === "subProcess") {
      const parsedInner = parseContainerElement(child, nname);
      if (parsedInner.nodes.length > 0) {
        subGraph = parsedInner;
      }
    }

    nodes.push({
      id: nid,
      name: nname,
      kind,
      documentation: meta.cleanDescription || rawDoc,
      laneName: laneByNodeId.get(nid) || meta.role,
      calledElement,
      subMethodId: meta.subMethodId || calledElement,
      taskId: meta.taskId,
      taskStatus: meta.taskStatus,
      workLineId: meta.workLineId,
      role: meta.role,
      inputs: meta.inputs,
      outputs: meta.outputs,
      checklist: meta.checklist,
      notes: meta.notes,
      subGraph,
    });
  }

  return { id, name, documentation, nodes, edges };
}

export function parseBpmnXml(xml: string, fallbackTitle = "ورک‌فلوی BPMN 2.0"): BpmnGraph {
  const trimmed = (xml || "").trim();
  if (!trimmed) {
    return {
      id: "Empty_Process",
      name: fallbackTitle,
      documentation: "",
      nodes: [],
      edges: [],
    };
  }

  try {
    const parser = new DOMParser();
    const doc = parser.parseFromString(trimmed, "application/xml");
    const parseErr = doc.querySelector("parsererror");
    if (parseErr) {
      return {
        id: "ParseError",
        name: fallbackTitle,
        documentation: "خطا در پارس XML: " + (parseErr.textContent || ""),
        nodes: [],
        edges: [],
      };
    }

    const allEls = Array.from(doc.getElementsByTagName("*"));
    const processEl = allEls.find((el) => localTagName(el) === "process");
    if (!processEl) {
      return {
        id: "NoProcess",
        name: fallbackTitle,
        documentation: "عنصر <bpmn:process> در سند XML یافت نشد.",
        nodes: [],
        edges: [],
      };
    }

    return parseContainerElement(processEl, fallbackTitle);
  } catch (e) {
    return {
      id: "Error",
      name: fallbackTitle,
      documentation: String(e),
      nodes: [],
      edges: [],
    };
  }
}

function getNodeDimensions(kind: BpmnNodeKind): { width: number; height: number } {
  switch (kind) {
    case "startEvent":
    case "endEvent":
    case "intermediateEvent":
      return { width: 46, height: 46 };
    case "exclusiveGateway":
    case "parallelGateway":
    case "inclusiveGateway":
      return { width: 58, height: 58 };
    case "subProcess":
    case "callActivity":
      return { width: 210, height: 94 };
    default:
      return { width: 192, height: 84 };
  }
}

export function layoutBpmnGraph(graph: BpmnGraph): PositionedBpmnGraph {
  if (graph.nodes.length === 0) {
    return {
      ...graph,
      nodes: [],
      edges: [],
      width: 600,
      height: 280,
    };
  }

  const nodeMap = new Map<string, BpmnNode>();
  const outgoing = new Map<string, string[]>();
  const incomingCount = new Map<string, number>();

  for (const n of graph.nodes) {
    nodeMap.set(n.id, n);
    outgoing.set(n.id, []);
    incomingCount.set(n.id, 0);
  }

  for (const e of graph.edges) {
    if (nodeMap.has(e.sourceRef) && nodeMap.has(e.targetRef)) {
      outgoing.get(e.sourceRef)!.push(e.targetRef);
      incomingCount.set(e.targetRef, (incomingCount.get(e.targetRef) || 0) + 1);
    }
  }

  // 1. Detect cycle back-edges via 3-color DFS so feedback loops never corrupt topological ranks
  const color = new Map<string, 0 | 1 | 2>(); // 0=white, 1=gray, 2=black
  const backEdgeKeys = new Set<string>();
  const edgeKey = (src: string, tgt: string) => `${src}-->${tgt}`;

  const roots = graph.nodes
    .filter((n) => (incomingCount.get(n.id) || 0) === 0 || n.kind === "startEvent")
    .map((n) => n.id);
  const dfsStarts = roots.length > 0 ? [...new Set(roots)] : [graph.nodes[0].id];

  const dfs = (u: string) => {
    color.set(u, 1);
    for (const v of outgoing.get(u) || []) {
      const c = color.get(v) || 0;
      if (c === 1) {
        backEdgeKeys.add(edgeKey(u, v));
      } else if (c === 0) {
        dfs(v);
      }
    }
    color.set(u, 2);
  };

  for (const s of dfsStarts) {
    if ((color.get(s) || 0) === 0) dfs(s);
  }
  for (const n of graph.nodes) {
    if ((color.get(n.id) || 0) === 0) dfs(n.id);
  }

  // Build DAG adjacency (excluding back-edges & self-loops)
  const dagOut = new Map<string, string[]>();
  const dagInDeg = new Map<string, number>();
  for (const n of graph.nodes) {
    dagOut.set(n.id, []);
    dagInDeg.set(n.id, 0);
  }
  for (const e of graph.edges) {
    if (!nodeMap.has(e.sourceRef) || !nodeMap.has(e.targetRef)) continue;
    if (e.sourceRef === e.targetRef || backEdgeKeys.has(edgeKey(e.sourceRef, e.targetRef))) {
      continue;
    }
    dagOut.get(e.sourceRef)!.push(e.targetRef);
    dagInDeg.set(e.targetRef, (dagInDeg.get(e.targetRef) || 0) + 1);
  }

  // 2. Kahn's topological longest-path rank assignment on the guaranteed-acyclic DAG
  const rankMap = new Map<string, number>();
  const kahnQueue: string[] = [];
  for (const n of graph.nodes) {
    if ((dagInDeg.get(n.id) || 0) === 0) {
      rankMap.set(n.id, n.kind === "startEvent" ? 0 : 0);
      kahnQueue.push(n.id);
    }
  }

  while (kahnQueue.length > 0) {
    const curr = kahnQueue.shift()!;
    const currRank = rankMap.get(curr) ?? 0;
    for (const next of dagOut.get(curr) || []) {
      const candidate = currRank + 1;
      const existing = rankMap.get(next);
      if (existing === undefined || candidate > existing) {
        rankMap.set(next, candidate);
      }
      const nextIn = (dagInDeg.get(next) || 1) - 1;
      dagInDeg.set(next, nextIn);
      if (nextIn === 0) {
        kahnQueue.push(next);
      }
    }
  }

  // Ensure any remaining nodes still get a deterministic rank
  let maxNonEndRank = 0;
  for (const n of graph.nodes) {
    const r = rankMap.get(n.id);
    if (r !== undefined && n.kind !== "endEvent" && r > maxNonEndRank) {
      maxNonEndRank = r;
    }
  }
  for (const n of graph.nodes) {
    if (!rankMap.has(n.id)) {
      rankMap.set(n.id, n.kind === "endEvent" ? maxNonEndRank + 1 : 1);
    } else if (n.kind === "endEvent" && (rankMap.get(n.id) ?? 0) <= maxNonEndRank && graph.nodes.length > 1) {
      rankMap.set(n.id, maxNonEndRank + 1);
    }
  }

  // Group nodes by rank
  const byRank = new Map<number, BpmnNode[]>();
  for (const n of graph.nodes) {
    const r = rankMap.get(n.id) ?? 0;
    if (!byRank.has(r)) byRank.set(r, []);
    byRank.get(r)!.push(n);
  }

  const sortedRanks = Array.from(byRank.keys()).sort((a, b) => a - b);
  const PAD_X = 64;
  // Reserve extra top headroom when feedback back-edges exist so loop arcs & labels never clip
  const backEdgeCount = graph.edges.filter((e) => {
    const sr = rankMap.get(e.sourceRef) ?? 0;
    const tr = rankMap.get(e.targetRef) ?? 0;
    return sr >= tr;
  }).length;
  const PAD_Y = backEdgeCount > 0 ? 74 + Math.min(backEdgeCount, 4) * 28 : 70;
  const COL_GAP = 72;
  const ROW_GAP = 42;

  // Compute max width per rank column & max rows
  const colWidths = new Map<number, number>();
  let maxColHeight = 0;

  for (const r of sortedRanks) {
    const list = byRank.get(r)!;
    let w = 60;
    let colH = 0;
    for (const n of list) {
      const dims = getNodeDimensions(n.kind);
      if (dims.width > w) w = dims.width;
      colH += dims.height;
    }
    colH += Math.max(0, list.length - 1) * ROW_GAP;
    colWidths.set(r, w);
    if (colH > maxColHeight) maxColHeight = colH;
  }

  const totalCanvasHeight = Math.max(320, maxColHeight + PAD_Y + 70);
  const positionedMap = new Map<string, PositionedBpmnNode>();
  let curX = PAD_X;

  for (const r of sortedRanks) {
    const list = byRank.get(r)!;
    const colW = colWidths.get(r)!;
    let colH = 0;
    for (const n of list) colH += getNodeDimensions(n.kind).height;
    colH += Math.max(0, list.length - 1) * ROW_GAP;

    let curY = Math.max(PAD_Y, PAD_Y + (maxColHeight - colH) / 2);

    list.forEach((n, rowIdx) => {
      const dims = getNodeDimensions(n.kind);
      const nodeX = curX + (colW - dims.width) / 2;
      const nodeY = curY;
      positionedMap.set(n.id, {
        ...n,
        x: nodeX,
        y: nodeY,
        width: dims.width,
        height: dims.height,
        rank: r,
        row: rowIdx,
      });
      curY += dims.height + ROW_GAP;
    });

    curX += colW + COL_GAP;
  }

  const totalCanvasWidth = Math.max(640, curX - COL_GAP + PAD_X);

  const positionedEdges: PositionedBpmnEdge[] = [];
  let backEdgeIndex = 0;
  for (const e of graph.edges) {
    const src = positionedMap.get(e.sourceRef);
    const tgt = positionedMap.get(e.targetRef);
    if (!src || !tgt) continue;

    const sx = src.x + src.width;
    const sy = src.y + src.height / 2;
    const tx = tgt.x;
    const ty = tgt.y + tgt.height / 2;

    let path: string;
    let labelX: number;
    let labelY: number;

    if (src.id !== tgt.id && tx >= sx - 10) {
      const dx = Math.max(32, (tx - sx) * 0.5);
      path = `M ${sx} ${sy} C ${sx + dx} ${sy}, ${tx - dx} ${ty}, ${tx} ${ty}`;
      labelX = (sx + tx) / 2;
      labelY = (sy + ty) / 2 - 10;
    } else if (src.id === tgt.id) {
      // Self-loop cleanly arched over the top of the node
      const loopY = Math.max(22, src.y - 36 - backEdgeIndex * 22);
      backEdgeIndex++;
      path = `M ${src.x + src.width * 0.7} ${src.y} C ${src.x + src.width * 0.9} ${loopY}, ${src.x + src.width * 0.1} ${loopY}, ${src.x + src.width * 0.3} ${src.y}`;
      labelX = src.x + src.width / 2;
      labelY = Math.max(14, loopY - 4);
    } else {
      // Back-edge loop over the top with non-overlapping lanes
      const laneOffset = 34 + backEdgeIndex * 26;
      backEdgeIndex++;
      const topY = Math.max(22, Math.min(src.y, tgt.y) - laneOffset);
      path = `M ${src.x + src.width / 2} ${src.y} C ${src.x + src.width / 2} ${topY}, ${tgt.x + tgt.width / 2} ${topY}, ${tgt.x + tgt.width / 2} ${tgt.y}`;
      labelX = (src.x + src.width / 2 + tgt.x + tgt.width / 2) / 2;
      labelY = Math.max(14, topY + 6);
    }

    positionedEdges.push({
      ...e,
      path,
      labelX,
      labelY,
    });
  }

  return {
    ...graph,
    nodes: Array.from(positionedMap.values()),
    edges: positionedEdges,
    width: totalCanvasWidth,
    height: totalCanvasHeight,
  };
}

function xmlEscape(raw: string): string {
  return (raw || "")
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

function renderTaskNodesXml(
  tasks: WorkEntity[],
  prefix: string,
  isSequential: boolean,
  emptyCompletedHint?: string
): string {
  const startId = `${prefix}_Start`;
  const endId = `${prefix}_End`;
  let xml = `      <bpmn:startEvent id="${startId}" name="شروع" />\n`;

  if (tasks.length === 0) {
    if (emptyCompletedHint) {
      const hintId = `${prefix}_DoneHint`;
      xml += `      <bpmn:task id="${hintId}" name="${xmlEscape(emptyCompletedHint)}">\n`;
      xml += `        <bpmn:documentation>TASK_STATUS:CLOSED</bpmn:documentation>\n`;
      xml += `      </bpmn:task>\n`;
      xml += `      <bpmn:sequenceFlow id="${prefix}_F_start" sourceRef="${startId}" targetRef="${hintId}" />\n`;
      xml += `      <bpmn:sequenceFlow id="${prefix}_F_end" sourceRef="${hintId}" targetRef="${endId}" />\n`;
    } else {
      xml += `      <bpmn:sequenceFlow id="${prefix}_F_empty" sourceRef="${startId}" targetRef="${endId}" />\n`;
    }
    xml += `      <bpmn:endEvent id="${endId}" name="پایان" />\n`;
    return xml;
  }

  const ordered = [...tasks].sort((a, b) => a.sequence_order - b.sequence_order);

  if (isSequential) {
    ordered.forEach((t, idx) => {
      const tid = `${prefix}_T_${idx}`;
      const docLines = [
        t.definition_of_done ? `شرط خروج (DoD): ${t.definition_of_done}` : "",
        t.first_physical_action ? `اقدام اول: ${t.first_physical_action}` : "",
        `دسته: ${t.category} | زمان: ${t.duration_minutes} دقیقه | انرژی: ${t.energy_tag} | ابزار: ${t.device_tag}`,
        `TASK_ID:${t.id}`,
        `TASK_STATUS:${t.status}`,
      ]
        .filter(Boolean)
        .join("\n");
      const tag = t.category === "DEPLOY" || t.category === "BACKEND" ? "serviceTask" : "userTask";
      xml += `      <bpmn:${tag} id="${tid}" name="${xmlEscape(`[${t.id}] ${t.title}`)}">\n`;
      xml += `        <bpmn:documentation>${xmlEscape(docLines)}</bpmn:documentation>\n`;
      xml += `      </bpmn:${tag}>\n`;
    });

    xml += `      <bpmn:sequenceFlow id="${prefix}_F_start" sourceRef="${startId}" targetRef="${prefix}_T_0" />\n`;
    for (let i = 0; i < ordered.length - 1; i++) {
      xml += `      <bpmn:sequenceFlow id="${prefix}_F_${i}" sourceRef="${prefix}_T_${i}" targetRef="${prefix}_T_${i + 1}" />\n`;
    }
    xml += `      <bpmn:sequenceFlow id="${prefix}_F_end" sourceRef="${prefix}_T_${ordered.length - 1}" targetRef="${endId}" />\n`;
  } else {
    // Parallel goal cluster
    const splitId = `${prefix}_Split`;
    const joinId = `${prefix}_Join`;
    xml += `      <bpmn:parallelGateway id="${splitId}" name="اجرای موازی" />\n`;
    xml += `      <bpmn:parallelGateway id="${joinId}" name="جمع‌بندی" />\n`;
    xml += `      <bpmn:sequenceFlow id="${prefix}_F_start" sourceRef="${startId}" targetRef="${splitId}" />\n`;
    ordered.forEach((t, idx) => {
      const tid = `${prefix}_T_${idx}`;
      const docLines = [
        t.definition_of_done ? `شرط خروج (DoD): ${t.definition_of_done}` : "",
        t.first_physical_action ? `اقدام اول: ${t.first_physical_action}` : "",
        `TASK_ID:${t.id}`,
        `TASK_STATUS:${t.status}`,
      ]
        .filter(Boolean)
        .join("\n");
      xml += `      <bpmn:userTask id="${tid}" name="${xmlEscape(`[${t.id}] ${t.title}`)}">\n`;
      xml += `        <bpmn:documentation>${xmlEscape(docLines)}</bpmn:documentation>\n`;
      xml += `      </bpmn:userTask>\n`;
      xml += `      <bpmn:sequenceFlow id="${prefix}_Fs_${idx}" sourceRef="${splitId}" targetRef="${tid}" />\n`;
      xml += `      <bpmn:sequenceFlow id="${prefix}_Fj_${idx}" sourceRef="${tid}" targetRef="${joinId}" />\n`;
    });
    xml += `      <bpmn:sequenceFlow id="${prefix}_F_end" sourceRef="${joinId}" targetRef="${endId}" />\n`;
  }

  xml += `      <bpmn:endEvent id="${endId}" name="پایان" />\n`;
  return xml;
}

/**
 * Generates a live BPMN 2.0 XML Work Map for the entire active project!
 * Each WorkLine is rendered as an expandable <bpmn:subProcess> containing its internal task workflow,
 * allowing click-to-inspect AND drill-down into every work stream!
 */
export function buildProjectWorkMapBpmn(dash: DashboardState): string {
  const workLines = (dash.work_lines ?? []).filter((w) => w.status !== "ARCHIVED");
  const openTasks = dash.open_tasks ?? [];
  const standaloneTasks = openTasks.filter((t) => !t.work_line_id);

  let body = `    <bpmn:startEvent id="Map_Start" name="شروع نقشه پروژه" />\n`;

  const groups: Array<{
    id: string;
    title: string;
    doc: string;
    tasks: WorkEntity[];
    isSeq: boolean;
    emptyCompletedHint?: string;
  }> = [];

  for (const wl of workLines) {
    const wlTasks = openTasks.filter((t) => t.work_line_id === wl.id);
    groups.push({
      id: `Sub_${wl.id.replace(/[^a-zA-Z0-9_]/g, "_")}`,
      title: `${wl.title} (${wl.completed_tasks}/${wl.total_tasks})`,
      doc: [
        wl.goal_statement || "مجموعه کارهای این لاین کاری",
        wl.outcome_title ? `دستاورد مرتبط (Outcome): ${wl.outcome_title}` : "",
        `حالت اجرا: ${wl.line_mode === "SEQUENTIAL_CHAIN" ? "زنجیره ترتیبی (دومینو)" : "خوشه هدف موازی"}`,
        `WORK_LINE_ID:${wl.id}`,
      ]
        .filter(Boolean)
        .join("\n"),
      tasks: wlTasks,
      isSeq: wl.line_mode === "SEQUENTIAL_CHAIN",
      emptyCompletedHint:
        wl.total_tasks > 0 && wlTasks.length === 0
          ? `تمام ${wl.total_tasks} گام این لاین تکمیل شده‌اند`
          : undefined,
    });
  }

  if (standaloneTasks.length > 0) {
    groups.push({
      id: "Sub_Standalone",
      title: `تسک‌های مستقل (${standaloneTasks.length})`,
      doc: "گام‌های مستقلی که هنوز به لاین کاری خاصی متصل نشده‌اند.",
      tasks: standaloneTasks,
      isSeq: false,
    });
  }

  if (groups.length === 0) {
    body += `    <bpmn:task id="Empty_Hint" name="هنوز لاین کاری یا تسکی تعریف نشده است" />\n`;
    body += `    <bpmn:sequenceFlow id="F_s0" sourceRef="Map_Start" targetRef="Empty_Hint" />\n`;
    body += `    <bpmn:sequenceFlow id="F_e0" sourceRef="Empty_Hint" targetRef="Map_End" />\n`;
  } else if (groups.length === 1) {
    const g = groups[0];
    body += `    <bpmn:subProcess id="${g.id}" name="${xmlEscape(g.title)}">\n`;
    body += `      <bpmn:documentation>${xmlEscape(g.doc)}</bpmn:documentation>\n`;
    body += renderTaskNodesXml(g.tasks, g.id, g.isSeq, g.emptyCompletedHint);
    body += `    </bpmn:subProcess>\n`;
    body += `    <bpmn:sequenceFlow id="F_s0" sourceRef="Map_Start" targetRef="${g.id}" />\n`;
    body += `    <bpmn:sequenceFlow id="F_e0" sourceRef="${g.id}" targetRef="Map_End" />\n`;
  } else {
    body += `    <bpmn:parallelGateway id="Map_Split" name="لاین‌های فعال پروژه" />\n`;
    body += `    <bpmn:parallelGateway id="Map_Join" name="تحقق دستاوردها" />\n`;
    body += `    <bpmn:sequenceFlow id="F_root" sourceRef="Map_Start" targetRef="Map_Split" />\n`;
    groups.forEach((g, idx) => {
      body += `    <bpmn:subProcess id="${g.id}" name="${xmlEscape(g.title)}">\n`;
      body += `      <bpmn:documentation>${xmlEscape(g.doc)}</bpmn:documentation>\n`;
      body += renderTaskNodesXml(g.tasks, g.id, g.isSeq, g.emptyCompletedHint);
      body += `    </bpmn:subProcess>\n`;
      body += `    <bpmn:sequenceFlow id="F_sp_${idx}" sourceRef="Map_Split" targetRef="${g.id}" />\n`;
      body += `    <bpmn:sequenceFlow id="F_jn_${idx}" sourceRef="${g.id}" targetRef="Map_Join" />\n`;
    });
    body += `    <bpmn:sequenceFlow id="F_end" sourceRef="Map_Join" targetRef="Map_End" />\n`;
  }

  body += `    <bpmn:endEvent id="Map_End" name="تحقق پروژه" />\n`;

  return `<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" id="Defs_Project_${xmlEscape(dash.active_project.id)}">
  <bpmn:process id="Process_${xmlEscape(dash.active_project.id)}" name="${xmlEscape(`نقشه کار پروژه ${dash.active_project.name}`)}">
    <bpmn:documentation>${xmlEscape(`نقشه جامع BPMN 2.0 برای ${workLines.length} لاین کاری و ${openTasks.length} تسک باز. روی هر مجموعه کار (SubProcess) کلیک یا دوبارکلیک کنید تا ورک‌فلوی داخلی آن باز شود.`)}</bpmn:documentation>
${body}  </bpmn:process>
</bpmn:definitions>`;
}

export function buildWorkLineBpmn(workLine: WorkLine, tasks: WorkEntity[]): string {
  const inner = renderTaskNodesXml(
    tasks,
    `WL_${workLine.id.replace(/[^a-zA-Z0-9_]/g, "_")}`,
    workLine.line_mode === "SEQUENTIAL_CHAIN"
  );
  return `<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" id="Defs_${xmlEscape(workLine.id)}">
  <bpmn:process id="Process_${xmlEscape(workLine.id)}" name="${xmlEscape(workLine.title)}">
    <bpmn:documentation>${xmlEscape(workLine.goal_statement || workLine.title)}</bpmn:documentation>
${inner}  </bpmn:process>
</bpmn:definitions>`;
}

export interface MethodDiagramDraftStep {
  id?: string;
  title: string;
  description?: string;
  element_type?: string;
  role?: string;
  condition_label?: string;
  sub_steps?: Array<{ id?: string; title: string; description?: string; role?: string }>;
}

function renderMethodStepNodesXml(steps: MethodDiagramDraftStep[], prefix: string): string {
  const startId = `${prefix}_Start`;
  const endId = `${prefix}_End`;
  const cleanSteps = steps.filter((s) => s.title.trim().length > 0);
  if (cleanSteps.length === 0) {
    return `    <bpmn:startEvent id="${startId}" name="شروع" />\n    <bpmn:endEvent id="${endId}" name="پایان" />\n    <bpmn:sequenceFlow id="${prefix}_F0" sourceRef="${startId}" targetRef="${endId}" />\n`;
  }

  let xml = `    <bpmn:startEvent id="${startId}" name="شروع" />\n`;
  const nodeIds = cleanSteps.map((s, idx) =>
    s.id?.trim() ? `${prefix}_${s.id.replace(/[^a-zA-Z0-9_]/g, "_")}` : `${prefix}_S${idx + 1}`
  );

  cleanSteps.forEach((step, idx) => {
    const nid = nodeIds[idx];
    const hasSub = step.sub_steps && step.sub_steps.length > 0;
    const tag = hasSub
      ? "subProcess"
      : step.element_type === "serviceTask" ||
        step.element_type === "exclusiveGateway" ||
        step.element_type === "parallelGateway" ||
        step.element_type === "userTask" ||
        step.element_type === "subProcess"
      ? step.element_type
      : "userTask";

    const docParts: string[] = [];
    if (step.description?.trim()) docParts.push(step.description.trim());
    if (step.role?.trim()) docParts.push(`نقش/مسئول: ${step.role.trim()}`);
    const docTag =
      docParts.length > 0
        ? `\n      <bpmn:documentation>${xmlEscape(docParts.join("\n"))}</bpmn:documentation>`
        : "";

    if (tag === "subProcess") {
      const inner = renderMethodStepNodesXml(
        (step.sub_steps ?? []).map((ss, sidx) => ({
          id: ss.id || `${idx + 1}_${sidx + 1}`,
          title: ss.title,
          description: ss.description,
          role: ss.role || step.role,
          element_type: "userTask",
        })),
        `${nid}_Sub`
      );
      xml += `    <bpmn:subProcess id="${nid}" name="${xmlEscape(step.title)}">${docTag}\n${inner}    </bpmn:subProcess>\n`;
    } else {
      xml += `    <bpmn:${tag} id="${nid}" name="${xmlEscape(step.title)}">${docTag}</bpmn:${tag}>\n`;
    }
  });

  xml += `    <bpmn:endEvent id="${endId}" name="پایان" />\n`;
  xml += `    <bpmn:sequenceFlow id="${prefix}_F_start" sourceRef="${startId}" targetRef="${nodeIds[0]}" />\n`;
  cleanSteps.forEach((step, idx) => {
    const src = nodeIds[idx];
    const tgt = idx + 1 < nodeIds.length ? nodeIds[idx + 1] : endId;
    const label = step.condition_label?.trim()
      ? ` name="${xmlEscape(step.condition_label.trim())}"`
      : "";
    xml += `    <bpmn:sequenceFlow id="${prefix}_F_${idx}" sourceRef="${src}" targetRef="${tgt}"${label} />\n`;
  });
  return xml;
}

export function buildMethodStepsBpmn(
  title: string,
  steps: MethodDiagramDraftStep[],
  objective?: string
): string {
  const inner = renderMethodStepNodesXml(steps, "Preview");
  return `<?xml version="1.0" encoding="UTF-8"?>
<bpmn:definitions xmlns:bpmn="http://www.omg.org/spec/BPMN/20100524/MODEL" id="Defs_Preview">
  <bpmn:process id="Process_Preview" name="${xmlEscape(title || "پیش‌نمایش دستورالعمل")}">
    <bpmn:documentation>${xmlEscape(objective || title || "پیش‌نمایش دیاگرام")}</bpmn:documentation>
${inner}  </bpmn:process>
</bpmn:definitions>`;
}
