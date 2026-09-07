(ns statusline-builder.layout
  (:require [statusline-builder.number :as number]
            [statusline-builder.width :as width]))

(defn runtime-preview-columns [columns]
  (max 0 (- (number/floor columns) 4)))

(defn- pieces-width [pieces]
  (reduce + 0 (map #(width/display-width (:text %)) pieces)))

(defn wrap-preview-segments [segments separator max-columns]
  (let [separator-width (width/display-width separator)
        step (fn [{:keys [lines line line-width]} pieces]
               (let [segment-width (pieces-width pieces)
                     projected (+ line-width separator-width segment-width)]
                 (cond
                   (empty? line) {:lines lines :line [{:pieces pieces :separator false}] :line-width segment-width}
                   (> projected max-columns) {:lines (conj lines line) :line [{:pieces pieces :separator false}] :line-width segment-width}
                   :else {:lines lines :line (conj line {:pieces pieces :separator true}) :line-width projected})))
        {:keys [lines line]} (reduce step {:lines [] :line [] :line-width 0} segments)]
    (if (seq line) (conj lines line) lines)))

(defn- block-rows [block separator max-columns]
  (let [lines (wrap-preview-segments (map :pieces block) separator max-columns)]
    (if (empty? lines)
      [{:standalone false :cells []}]
      (loop [lines lines, entries block, rows []]
        (if-let [line (first lines)]
          (let [cells (mapv (fn [cell entry] {:entry entry :separator (:separator cell)}) line entries)]
            (recur (rest lines) (drop (count line) entries) (conj rows {:standalone false :cells cells})))
          rows)))))

(defn layout-rows [entries separator max-columns]
  (if (empty? entries)
    []
    (let [flush (fn [rows block]
                  (if (or (empty? rows) (seq block))
                    (into rows (block-rows block separator max-columns))
                    rows))
          [rows block] (reduce (fn [[rows block] entry]
                                 (if (:standalone entry)
                                   [(conj (flush rows block) {:standalone true :cells [{:entry entry :separator false}]}) []]
                                   [rows (conj block entry)]))
                               [[] []]
                               entries)]
      (flush rows block))))

(defn describe-line-fill [entries separator max-columns]
  (let [rows (layout-rows entries separator max-columns)
        separator-width (width/display-width separator)
        row-width (fn [row]
                    (reduce + 0 (map (fn [cell] (+ (if (:separator cell) separator-width 0) (pieces-width (:pieces (:entry cell))))) (:cells row))))
        widest (reduce max 0 (map row-width rows))
        count-text (if (= 1 (count rows)) "1 row" (str (count rows) " rows"))]
    (str widest " of " max-columns " cols, " count-text)))

(defn nearest-drop-slot [boxes x y]
  (when (seq boxes)
    (let [row-distance (fn [{:keys [rect]}]
                         (cond
                           (< y (:top rect)) (- (:top rect) y)
                           (> y (:bottom rect)) (- y (:bottom rect))
                           :else 0))
          closest (apply min (map row-distance boxes))
          row (filter #(= closest (row-distance %)) boxes)
          centre (fn [{:keys [rect]}] (+ (:left rect) (/ (- (:right rect) (:left rect)) 2)))
          nearest (reduce (fn [best box] (if (< (abs (- (centre box) x)) (abs (- (centre best) x))) box best)) row)]
      {:node (:node nearest) :before (< x (centre nearest))})))
