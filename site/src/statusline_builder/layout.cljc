(ns statusline-builder.layout
  (:require [clojure.string :as str]
            [statusline-builder.number :as number]
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

(defn- graphemes-of [pieces]
  (vec (for [[index piece] (map-indexed vector pieces)
             grapheme (width/graphemes (:text piece))]
         {:grapheme grapheme :piece index :width (width/display-width grapheme)})))

(defn- rebuild-pieces [pieces cells]
  (let [texts (reduce (fn [texts {:keys [piece grapheme]}] (update texts piece str grapheme))
                      (vec (repeat (count pieces) ""))
                      cells)]
    (mapv (fn [piece text] (assoc piece :text text)) pieces texts)))

(defn truncate-pieces [pieces max-columns]
  (let [cells (graphemes-of pieces)]

    (if (<= (reduce + 0 (map :width cells)) max-columns)
      pieces
      (let [room (max 0 (dec max-columns))
            kept (loop [remaining cells, kept [], used 0]
                   (let [cell (first remaining)]

                     (if (or (nil? cell) (> (+ used (:width cell)) room))
                       kept
                       (recur (rest remaining) (conj kept cell) (+ used (:width cell))))))
            ellipsis {:grapheme "…" :piece (if (seq kept) (:piece (peek kept)) 0) :width 1}]
        (rebuild-pieces pieces (conj kept ellipsis))))))

(defn- words [cells]
  (->> cells
       (partition-by #(str/blank? (:grapheme %)))
       (remove #(str/blank? (:grapheme (first %))))))

(defn wrap-pieces [pieces max-columns]
  (let [word-width (fn [word] (reduce + 0 (map :width word)))
        glue (fn [out grapheme width] (conj out {:grapheme grapheme :piece (:piece (peek out)) :width width}))
        step (fn [{:keys [out line-width]} word]
               (let [word-w (word-width word)]
                 (cond
                   (empty? out) {:out (vec word) :line-width word-w}
                   (> (+ line-width 1 word-w) max-columns) {:out (into (glue out "\n" 0) word) :line-width word-w}
                   :else {:out (into (glue out " " 1) word) :line-width (+ line-width 1 word-w)})))]
    (rebuild-pieces pieces (:out (reduce step {:out [] :line-width 0} (words (graphemes-of pieces)))))))

(defn- fit-standalone [entry max-columns]
  (if (pos? max-columns)
    (update entry :pieces (if (= :truncate (:overflow entry)) truncate-pieces wrap-pieces) max-columns)
    entry))

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
                                   [(conj (flush rows block) {:standalone true :cells [{:entry (fit-standalone entry max-columns) :separator false}]}) []]
                                   [rows (conj block entry)]))
                               [[] []]
                               entries)]
      (flush rows block))))

(defn describe-line-fill [entries separator max-columns]
  (let [rows (layout-rows entries separator max-columns)
        row-text (fn [row]
                   (apply str (mapcat (fn [cell] (cons (if (:separator cell) separator "") (map :text (:pieces (:entry cell))))) (:cells row))))
        row-width (fn [row] (reduce max 0 (map width/display-width (str/split (row-text row) #"\n"))))
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
