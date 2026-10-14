import { motion } from "framer-motion";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  type CSSProperties,
  type DragEvent as ReactDragEvent,
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent,
  type RefObject,
} from "react";

import {
  darken,
  lighten,
  normalizeHexColor,
  readableTextColor,
  withAlpha,
} from "@/lib/colors";
import type { GameState } from "@/types";

type CellCoord = { row: number; col: number };
type BoundaryBreaks = {
  top: boolean;
  right: boolean;
  bottom: boolean;
  left: boolean;
};

type ViewportSize = {
  width: number;
  height: number;
};

type PointerCoords = {
  x: number;
  y: number;
};

type BoardMetrics = {
  cellSize: number;
  baseWidth: number;
  baseHeight: number;
  needsOverflow: boolean;
  viewportHeight: number;
  fitScale: number;
};

type BoardPointerHandlers = {
  onPointerDown?: (event: ReactPointerEvent<HTMLDivElement>) => void;
  onPointerMove?: (event: ReactPointerEvent<HTMLDivElement>) => void;
  onPointerUp?: (event: ReactPointerEvent<HTMLDivElement>) => void;
  onPointerCancel?: (event: ReactPointerEvent<HTMLDivElement>) => void;
  onLostPointerCapture?: (event: ReactPointerEvent<HTMLDivElement>) => void;
  onContextMenu?: (event: ReactMouseEvent<HTMLDivElement>) => void;
  onDragStart?: (event: ReactDragEvent<HTMLDivElement>) => void;
};

type PanGesture = {
  startScale: number;
  startPan: { x: number; y: number };
  startDistance: number | null;
  startCentroid: PointerCoords;
  anchor: PointerCoords;
};

const CELL_GAP_PX = 2;
const MIN_CELL_SIZE = 34;
const SMALL_BOARD_MAX_CELL_SIZE = 36;
const MEDIUM_BOARD_MAX_CELL_SIZE = 34;
const LARGE_BOARD_MAX_CELL_SIZE = 36;
const DEFAULT_CELL_SIZE = 44;
const MIN_VIEWPORT_HEIGHT = 280;
const MAX_VIEWPORT_HEIGHT = 720;
const MAX_ZOOM_SCALE = 2;
const ZOOM_STEP = 0.16;
const BOARD_VIEWPORT_GUTTER = 24;
const BOARD_FRAME_VERTICAL_CHROME = 26;

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), max);
}

function getCentroid(points: PointerCoords[]) {
  const total = points.reduce(
    (sum, point) => ({ x: sum.x + point.x, y: sum.y + point.y }),
    { x: 0, y: 0 }
  );
  return {
    x: total.x / points.length,
    y: total.y / points.length,
  };
}

function getDistance(a: PointerCoords, b: PointerCoords) {
  return Math.hypot(a.x - b.x, a.y - b.y);
}

function getAnchorFromCentroid(
  centroid: PointerCoords,
  pan: { x: number; y: number },
  scale: number
) {
  return {
    x: (centroid.x - pan.x) / scale,
    y: (centroid.y - pan.y) / scale,
  };
}

function clampPan(
  pan: { x: number; y: number },
  scale: number,
  metrics: BoardMetrics,
  viewportSize: ViewportSize
) {
  const scaledWidth = metrics.baseWidth * scale;
  const scaledHeight = metrics.baseHeight * scale;

  const clampAxis = (offset: number, scaledSize: number, viewportValue: number) => {
    if (scaledSize <= viewportValue) {
      return (viewportValue - scaledSize) / 2;
    }

    return clamp(offset, viewportValue - scaledSize, 0);
  };

  return {
    x: clampAxis(pan.x, scaledWidth, viewportSize.width),
    y: clampAxis(pan.y, scaledHeight, viewportSize.height),
  };
}

function getPreferredMaxCellSize(gridWidth: number, gridHeight: number) {
  const largestSide = Math.max(gridWidth, gridHeight);
  if (largestSide <= 8) {
    return SMALL_BOARD_MAX_CELL_SIZE;
  }
  if (largestSide <= 12) {
    return MEDIUM_BOARD_MAX_CELL_SIZE;
  }
  return LARGE_BOARD_MAX_CELL_SIZE;
}

function calculateMetrics(
  gridWidth: number,
  gridHeight: number,
  viewportSize: ViewportSize
): BoardMetrics {
  if (viewportSize.width <= 0) {
    const baseWidth = gridWidth * DEFAULT_CELL_SIZE + Math.max(gridWidth - 1, 0) * CELL_GAP_PX;
    const baseHeight = gridHeight * DEFAULT_CELL_SIZE + Math.max(gridHeight - 1, 0) * CELL_GAP_PX;

    return {
      cellSize: DEFAULT_CELL_SIZE,
      baseWidth,
      baseHeight,
      needsOverflow: gridWidth > 8,
      viewportHeight: MIN_VIEWPORT_HEIGHT,
      fitScale: 1,
    };
  }

  const preferredMaxCellSize = getPreferredMaxCellSize(gridWidth, gridHeight);
  const fitWidth =
    (viewportSize.width - Math.max(gridWidth - 1, 0) * CELL_GAP_PX) / Math.max(gridWidth, 1);
  const fitHeight =
    (viewportSize.height - Math.max(gridHeight - 1, 0) * CELL_GAP_PX) / Math.max(gridHeight, 1);
  const desiredCell = Math.min(fitWidth, fitHeight);
  const cellSize = Math.max(
    Math.floor(Math.min(Math.max(desiredCell, MIN_CELL_SIZE), preferredMaxCellSize)),
    MIN_CELL_SIZE
  );
  const fitScale = desiredCell < cellSize ? clamp(desiredCell / cellSize, 0.55, 1) : 1;
  const needsOverflow = fitScale < 0.999;
  const baseWidth = gridWidth * cellSize + Math.max(gridWidth - 1, 0) * CELL_GAP_PX;
  const baseHeight = gridHeight * cellSize + Math.max(gridHeight - 1, 0) * CELL_GAP_PX;

  return {
    cellSize,
    baseWidth,
    baseHeight,
    needsOverflow,
    viewportHeight: viewportSize.height,
    fitScale,
  };
}

type GameBoardProps = {
  grid: NonNullable<GameState["grid"]>;
  selectedCells: CellCoord[];
  foundWords: Set<string>;
  foundCellOwners: Map<string, string[]>;
  foundCellWordKeys: Map<string, string[]>;
  playersById: Map<string, GameState["players"][number]>;
  currentUserId: string | null;
  highlightColor: string;
  interactive: boolean;
  gridRef?: RefObject<HTMLDivElement | null>;
  boardPointerHandlers?: BoardPointerHandlers;
  getPlayerColor: (
    playerId: string,
    playersById: Map<string, GameState["players"][number]>,
    currentUserId: string | null,
    currentUserColor: string
  ) => string;
  buildCellGradient: (color: string, seed: string) => string;
  buildStackedMonoHighlight: (color: string, overlapCount: number, seed: string) => string;
  getBoundaryBreaks: (
    row: number,
    col: number,
    currentWordKeys: string[],
    wordOwnership: Map<string, string[]>
  ) => BoundaryBreaks;
  buildBoundaryOverlay: (baseBackground: string, color: string, breaks: BoundaryBreaks) => string;
};



export default function GameBoard({
  grid,
  selectedCells,
  foundWords,
  foundCellOwners,
  foundCellWordKeys,
  playersById,
  currentUserId,
  highlightColor,
  interactive,
  gridRef,
  boardPointerHandlers,
  getPlayerColor,
  buildCellGradient,
  buildStackedMonoHighlight,
  getBoundaryBreaks,
  buildBoundaryOverlay,
}: GameBoardProps) {
  const selectedCellKeys = new Set(selectedCells.map((cell) => `${cell.row},${cell.col}`));
  const shellRef = useRef<HTMLDivElement>(null);
  const measureRef = useRef<HTMLDivElement>(null);
  const viewportRef = useRef<HTMLDivElement>(null);
  const pointersRef = useRef<Map<number, PointerCoords>>(new Map());
  const gestureRef = useRef<PanGesture | null>(null);
  const [viewportSize, setViewportSize] = useState<ViewportSize>({
    width: 0,
    height: MIN_VIEWPORT_HEIGHT,
  });
  const [zoomScale, setZoomScale] = useState(1);
  const [viewPan, setViewPan] = useState({ x: 0, y: 0 });
  const [showGestureHint, setShowGestureHint] = useState(false);

  const metrics = useMemo(
    () => calculateMetrics(grid.width, grid.height, viewportSize),
    [grid.height, grid.width, viewportSize]
  );
  const usingViewportTransform =
    metrics.needsOverflow || zoomScale > metrics.fitScale + 0.01;
  const viewportBounds = useMemo<ViewportSize>(() => {
    const fitWidth = Math.ceil(metrics.baseWidth * metrics.fitScale);
    const fitHeight = Math.ceil(metrics.baseHeight * metrics.fitScale);

    return {
      width: usingViewportTransform
        ? viewportSize.width
        : Math.min(viewportSize.width, fitWidth),
      height: usingViewportTransform
        ? Math.min(metrics.baseHeight, metrics.viewportHeight)
        : Math.min(metrics.viewportHeight, fitHeight),
    };
  }, [
    metrics.baseHeight,
    metrics.baseWidth,
    metrics.fitScale,
    metrics.viewportHeight,
    usingViewportTransform,
    viewportSize.width,
  ]);
  const resetPan = useMemo(
    () => clampPan({ x: 0, y: 0 }, metrics.fitScale, metrics, viewportBounds),
    [metrics, viewportBounds]
  );
  const canPanOrZoom = interactive && usingViewportTransform;
  const canResetView =
    Math.abs(zoomScale - metrics.fitScale) > 0.01 ||
    Math.abs(viewPan.x - resetPan.x) > 1 ||
    Math.abs(viewPan.y - resetPan.y) > 1;

  const syncViewportSize = useCallback(() => {
    if (!shellRef.current || !measureRef.current || typeof window === "undefined") {
      return;
    }

    const shellRect = shellRef.current.getBoundingClientRect();
    const measureRect = measureRef.current.getBoundingClientRect();
    setViewportSize({
      width: measureRect.width,
      height: clamp(
        window.innerHeight - shellRect.top - BOARD_VIEWPORT_GUTTER - BOARD_FRAME_VERTICAL_CHROME,
        MIN_VIEWPORT_HEIGHT,
        MAX_VIEWPORT_HEIGHT
      ),
    });
  }, []);

  useEffect(() => {
    syncViewportSize();

    if (typeof window === "undefined") {
      return;
    }

    const handleResize = () => syncViewportSize();
    window.addEventListener("resize", handleResize);

    if ((!shellRef.current && !measureRef.current) || typeof ResizeObserver === "undefined") {
      return () => window.removeEventListener("resize", handleResize);
    }

    const observer = new ResizeObserver(() => {
      syncViewportSize();
    });
    if (shellRef.current) {
      observer.observe(shellRef.current);
    }
    if (measureRef.current) {
      observer.observe(measureRef.current);
    }

    return () => {
      observer.disconnect();
      window.removeEventListener("resize", handleResize);
    };
  }, [syncViewportSize]);

  useEffect(() => {
    setZoomScale(metrics.fitScale);
    setViewPan(resetPan);
    pointersRef.current.clear();
    gestureRef.current = null;
  }, [grid.height, grid.width, metrics.fitScale, resetPan]);

  useEffect(() => {
    setViewPan((current) => clampPan(current, zoomScale, metrics, viewportBounds));
  }, [metrics, viewportBounds, zoomScale]);

  useEffect(() => {
    if (!interactive || !canPanOrZoom || typeof window === "undefined") {
      setShowGestureHint(false);
      return;
    }

    setShowGestureHint(true);
    const timeout = window.setTimeout(() => setShowGestureHint(false), 5000);
    return () => window.clearTimeout(timeout);
  }, [canPanOrZoom, interactive]);

  const resetView = useCallback(() => {
    setZoomScale(metrics.fitScale);
    setViewPan(resetPan);
  }, [metrics.fitScale, resetPan]);

  const updatePanGesture = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>, points: PointerCoords[]) => {
      if (!interactive || points.length < 2) {
        return;
      }

      const centroid = getCentroid(points);
      const distance = getDistance(points[0], points[1]);

      if (!gestureRef.current) {
        gestureRef.current = {
          startScale: zoomScale,
          startPan: viewPan,
          startDistance: distance,
          startCentroid: centroid,
          anchor: getAnchorFromCentroid(centroid, viewPan, zoomScale),
        };
        boardPointerHandlers?.onPointerCancel?.(event);
      }

      const gesture = gestureRef.current;
      const scaleRatio =
        gesture.startDistance && gesture.startDistance > 0
          ? distance / gesture.startDistance
          : 1;
      const nextScale = clamp(gesture.startScale * scaleRatio, metrics.fitScale, MAX_ZOOM_SCALE);
      const rawPan = {
        x:
          gesture.startPan.x +
          (centroid.x - gesture.startCentroid.x) +
          gesture.anchor.x * (gesture.startScale - nextScale),
        y:
          gesture.startPan.y +
          (centroid.y - gesture.startCentroid.y) +
          gesture.anchor.y * (gesture.startScale - nextScale),
      };

      setZoomScale(nextScale);
      setViewPan(clampPan(rawPan, nextScale, metrics, viewportBounds));
    },
    [
      boardPointerHandlers,
      interactive,
      metrics,
      viewPan,
      viewportBounds,
      zoomScale,
    ]
  );

  const handlePointerDown = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      event.currentTarget.setPointerCapture?.(event.pointerId);
      const rect = viewportRef.current?.getBoundingClientRect();
      pointersRef.current.set(event.pointerId, {
        x: event.clientX - (rect?.left ?? 0),
        y: event.clientY - (rect?.top ?? 0),
      });

      const points = [...pointersRef.current.values()];
      if (interactive && points.length >= 2) {
        updatePanGesture(event, points);
        return;
      }

      boardPointerHandlers?.onPointerDown?.(event);
    },
    [boardPointerHandlers, interactive, updatePanGesture]
  );

  const handlePointerMove = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      if (pointersRef.current.has(event.pointerId)) {
        const rect = viewportRef.current?.getBoundingClientRect();
        pointersRef.current.set(event.pointerId, {
          x: event.clientX - (rect?.left ?? 0),
          y: event.clientY - (rect?.top ?? 0),
        });
      }

      const points = [...pointersRef.current.values()];
      if (interactive && (gestureRef.current || points.length >= 2)) {
        updatePanGesture(event, points);
        return;
      }

      boardPointerHandlers?.onPointerMove?.(event);
    },
    [boardPointerHandlers, interactive, updatePanGesture]
  );

  const handlePointerUp = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      const wasGesture = Boolean(gestureRef.current);
      pointersRef.current.delete(event.pointerId);
      if (event.currentTarget.hasPointerCapture?.(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }

      if (wasGesture) {
        if (pointersRef.current.size < 2) {
          gestureRef.current = null;
        }
        return;
      }

      boardPointerHandlers?.onPointerUp?.(event);
    },
    [boardPointerHandlers]
  );

  const handlePointerCancel = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      pointersRef.current.delete(event.pointerId);
      if (event.currentTarget.hasPointerCapture?.(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
      if (gestureRef.current) {
        if (pointersRef.current.size < 2) {
          gestureRef.current = null;
        }
        return;
      }

      boardPointerHandlers?.onPointerCancel?.(event);
    },
    [boardPointerHandlers]
  );

  const handleLostPointerCapture = useCallback(
    (event: ReactPointerEvent<HTMLDivElement>) => {
      pointersRef.current.clear();
      gestureRef.current = null;
      boardPointerHandlers?.onLostPointerCapture?.(event);
    },
    [boardPointerHandlers]
  );

  const gridStyle = useMemo(
    () => ({
      gridTemplateColumns: `repeat(${grid.width}, var(--cell-size))`,
      ["--cell-size" as string]: `${metrics.cellSize}px`,
    }),
    [grid.width, metrics.cellSize]
  );
  const stageStyle = useMemo(
    () => ({
      width: `${metrics.baseWidth}px`,
      height: `${metrics.baseHeight}px`,
      transform: `translate3d(${viewPan.x}px, ${viewPan.y}px, 0) scale(${zoomScale})`,
      transformOrigin: "top left",
    }),
    [metrics.baseHeight, metrics.baseWidth, viewPan.x, viewPan.y, zoomScale]
  );
  const stepZoom = useCallback(
    (direction: "in" | "out") => {
      const nextScale = clamp(
        zoomScale + (direction === "in" ? ZOOM_STEP : -ZOOM_STEP),
        metrics.fitScale,
        MAX_ZOOM_SCALE
      );
      const viewportCenter = {
        x: viewportBounds.width / 2,
        y: viewportBounds.height / 2,
      };
      const anchor = getAnchorFromCentroid(viewportCenter, viewPan, zoomScale);
      const rawPan = {
        x: viewportCenter.x - anchor.x * nextScale,
        y: viewportCenter.y - anchor.y * nextScale,
      };
      setZoomScale(nextScale);
      setViewPan(clampPan(rawPan, nextScale, metrics, viewportBounds));
    },
    [metrics, viewportBounds, viewPan, zoomScale]
  );

  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-center justify-end gap-1.5 px-0.5">
        {interactive && (
          <>
            <button
              type="button"
              onClick={() => stepZoom("out")}
              className="min-h-8 min-w-8 rounded-full border border-border/70 px-2 text-sm font-semibold text-text-secondary transition hover:bg-bg-secondary hover:text-text"
              aria-label="Zoom out"
            >
              -
            </button>
            <button
              type="button"
              onClick={() => stepZoom("in")}
              className="min-h-8 min-w-8 rounded-full border border-border/70 px-2 text-sm font-semibold text-text-secondary transition hover:bg-bg-secondary hover:text-text"
              aria-label="Zoom in"
            >
              +
            </button>
          </>
        )}
        {canResetView && (
          <button
            type="button"
            onClick={resetView}
            className="min-h-8 rounded-full border border-border/70 px-2.5 text-[11px] font-semibold uppercase tracking-[0.12em] text-text-secondary transition hover:bg-bg-secondary hover:text-text"
          >
            Fit
          </button>
        )}
      </div>

      <div
        ref={shellRef}
        className="game-board-shell overflow-hidden"
        style={{
          touchAction: canPanOrZoom ? "none" : "auto",
        }}
      >
        <div className="relative w-full overflow-hidden">
          <div ref={measureRef} className="w-full">
            <div
              ref={viewportRef}
              className="relative mx-auto overflow-hidden"
              style={{
                width: `${Math.ceil(viewportBounds.width)}px`,
                height: `${Math.ceil(viewportBounds.height)}px`,
              }}
            >
              <div className="absolute left-0 top-0" style={stageStyle}>
                <div
                  ref={interactive ? gridRef : undefined}
                  className={`game-board inline-grid w-max gap-0.5 select-none ${
                    interactive ? "game-board--interactive" : ""
                  }`}
                  style={gridStyle}
                  onPointerDown={handlePointerDown}
                  onPointerMove={handlePointerMove}
                  onPointerUp={handlePointerUp}
                  onPointerCancel={handlePointerCancel}
                  onLostPointerCapture={handleLostPointerCapture}
                  onContextMenu={(event) => boardPointerHandlers?.onContextMenu?.(event)}
                  onDragStart={(event) => boardPointerHandlers?.onDragStart?.(event)}
                >
                  {grid.cells.map((row, r) =>
                    row.map((cell, c) => {
                      const cellKey = `${r},${c}`;
                      const isSelected = selectedCellKeys.has(cellKey);
                      const owners = foundCellOwners.get(cellKey) ?? [];
                      const found = owners.length > 0;
                      const primaryOwner = owners.includes(currentUserId ?? "")
                        ? currentUserId
                        : owners[0];

                      let cellStyle: CSSProperties | undefined;
                      let textColor: string | undefined;

                      if (isSelected) {
                        const base = highlightColor;
                        cellStyle = {
                          background: buildCellGradient(base, `${cellKey}:${selectedCells.length}`),
                          boxShadow: `0 0 0 1px ${withAlpha(base, 0.35)} inset, 0 10px 18px ${withAlpha(base, 0.18)}`,
                        };
                        textColor = readableTextColor(base);
                      } else if (found && primaryOwner) {
                        const colors = owners.map((ownerId) =>
                          getPlayerColor(ownerId, playersById, currentUserId, highlightColor)
                        );
                        const normalizedColors = colors.map((color) => normalizeHexColor(color, color));
                        const distinctColors = [...new Set(normalizedColors)];
                        const allSameColor = distinctColors.length === 1;
                        const currentWordKeys = foundCellWordKeys.get(cellKey) ?? [];
                        const base = distinctColors[0];
                        const seed = currentWordKeys.join("|") || `${cellKey}:${owners.join("|")}`;
                        const accent = allSameColor && normalizedColors.length > 1
                          ? darken(base, Math.min(0.16 + (normalizedColors.length - 2) * 0.06, 0.34))
                          : base;
                        const baseBackground = normalizedColors.length > 1
                          ? allSameColor
                            ? buildStackedMonoHighlight(base, normalizedColors.length, seed)
                            : `linear-gradient(${100 + (seed.length % 80)}deg, ${lighten(distinctColors[0], 0.12)} 0%, ${distinctColors[0]} 42%, ${distinctColors[1]} 58%, ${darken(distinctColors[1], 0.08)} 100%)`
                          : buildCellGradient(base, seed);
                        const background = buildBoundaryOverlay(
                          baseBackground,
                          accent,
                          getBoundaryBreaks(r, c, currentWordKeys, foundCellWordKeys)
                        );
                        cellStyle = {
                          background,
                          boxShadow: `0 0 0 1px ${withAlpha(accent, 0.24)} inset, 0 8px 18px ${withAlpha(accent, allSameColor && normalizedColors.length > 1 ? 0.22 : 0.16)}`,
                        };
                        textColor = readableTextColor(accent);
                      }

                      return (
                        <motion.div
                          key={`${r}-${c}`}
                          data-row={r}
                          data-col={c}
                          className={`game-board-cell flex h-[var(--cell-size)] w-[var(--cell-size)] min-w-[var(--cell-size)] items-center justify-center rounded-[0.8rem] text-sm font-bold sm:text-base transition-shadow ${
                            isSelected
                              ? "scale-105"
                              : found
                                ? ""
                                : "bg-cell-bg text-text border border-border"
                          }`}
                          style={cellStyle ? { ...cellStyle, color: textColor } : undefined}
                          animate={isSelected ? { scale: 1.05 } : { scale: 1 }}
                          transition={{ duration: 0.1 }}
                        >
                          {cell}
                        </motion.div>
                      );
                    })
                  )}
                </div>
              </div>
            </div>
          </div>
        </div>
      </div>

    </div>
  );
}
