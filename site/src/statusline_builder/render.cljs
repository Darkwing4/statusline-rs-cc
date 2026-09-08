(ns statusline-builder.render
  (:require [statusline-builder.actions :as actions]
            [statusline-builder.colour :as colour]
            [statusline-builder.controls :as controls]
            [statusline-builder.dom :as dom :refer [$ el]]
            [statusline-builder.drag-drop :as drag-drop]
            [statusline-builder.editor-state :as es]
            [statusline-builder.layout :as layout]
            [statusline-builder.preview :as preview]
            [statusline-builder.segment :as segment]
            [statusline-builder.width :as width]))

(def ^:private animated-pieces ".piece, .ghost-piece, .shelf-piece")

(defn- piece-button [class segment module]
  (let [node (el "button" class)]
    (set! (.-type node) "button")
    (.setAttribute node "data-segment-id" (:id segment))
    (.setAttribute node "data-module-id" (:module-id segment))
    (when (= (:id segment) (:selected-id @es/state))
      (.add (.-classList node) "is-selected"))
    (.addEventListener node "click" #(actions/select! (:id segment)))
    (drag-drop/make-draggable! node {:kind :line :id (:id segment)} "move" (:label module))
    node))

(defn- piece-node [state segment pieces]
  (let [module (es/segment-module state segment)
        node (piece-button "piece" segment module)]
    (set! (.-title node) (:label module))
    (.setAttribute node "aria-label" (str (:label module) " on the line"))
    (cond
      (segment/line-break? segment) (.add (.-classList node) "is-break")
      (every? #(zero? (width/display-width (:text %))) pieces) (.add (.-classList node) "is-blank"))
    (doseq [{:keys [text colour]} pieces]
      (let [span (el "span" "" text)]
        (set! (.. span -style -color) colour)
        (.append node span)))
    node))

(defn- separator-node [state]
  (let [node (el "button" "sep" (:separator state))]
    (set! (.-type node) "button")
    (set! (.-title node) "Separator")
    (.setAttribute node "aria-label" "Separator settings")
    (set! (.. node -style -color) (colour/css (:separator-color state) 50))
    (when (= es/line-selection (:selected-id state))
      (.add (.-classList node) "is-selected"))
    (.addEventListener node "click" #(actions/select! es/line-selection))
    node))

(defn- row-node [state row]
  (let [node (el "span" "line-row")]
    (when (:standalone row)
      (.add (.-classList node) "is-standalone"))
    (doseq [{:keys [entry separator]} (:cells row)]
      (when separator
        (.append node (separator-node state)))
      (.append node (piece-node state (:segment entry) (:pieces entry))))
    node))

(defn- hidden-pieces! [state entries]
  (let [container ($ "hiddenPieces")]
    (.replaceChildren container)
    (when (seq entries)
      (.append container (el "span" "hidden-pieces-label" "Silent in this session:"))
      (doseq [{:keys [segment]} entries]
        (let [module (es/segment-module state segment)
              node (piece-button "ghost-piece" segment module)]
          (set! (.-textContent node) (:label module))
          (set! (.-title node) (:description module))
          (.append container node))))))

(defn line! [state]
  (let [session (es/scenario state)
        drawn (map (fn [entry] {:segment entry
                                :pieces (preview/pieces entry session (:separator-color state))
                                :standalone (segment/standalone? entry)
                                :line-break (segment/line-break? entry)
                                :overflow (segment/overflow entry)})
                   (:segments state))
        rendered (filter #(seq (:pieces %)) drawn)
        max-columns (layout/runtime-preview-columns (:terminal-width state))
        canvas ($ "lineCanvas")]
    (hidden-pieces! state (remove #(seq (:pieces %)) drawn))
    (.replaceChildren canvas)
    (.setProperty (.-style ($ "lineScreen")) "--terminal-columns" (str (:terminal-width state)))
    (if (empty? rendered)
      (.append canvas (el "span" "line-empty" "Nothing on the line yet — drag a piece up from below."))
      (let [rows (mapv #(row-node state %) (layout/layout-rows rendered (:separator state) max-columns))]
        (doseq [row rows]
          (.append canvas row))
        (.append (peek rows) (el "span" "line-cursor"))))
    (dom/set-text! "terminalWidthValue" (str (:terminal-width state)))
    (set! (.-value ($ "terminalWidth")) (str (:terminal-width state)))
    (dom/set-text! "lineDimensions" (layout/describe-line-fill rendered (:separator state) max-columns))))

(defn selection! [state]
  (let [target (:selected-id state)]
    (doseq [node (dom/nodes ".piece, .ghost-piece")]
      (.toggle (.-classList node) "is-selected" (= target (dom/segment-id node))))
    (doseq [node (array-seq (.querySelectorAll ($ "lineCanvas") ".sep"))]
      (.toggle (.-classList node) "is-selected" (= target es/line-selection)))))

(defn shelf! [state]
  (let [available (es/shelf-modules state)
        shelf ($ "shelf")]
    (.replaceChildren shelf)
    (if (empty? available)
      (.append shelf (el "p" "shelf-empty" "Every piece is on the line."))
      (doseq [module available]
        (let [node (el "button" "shelf-piece")]
          (set! (.-type node) "button")
          (.setAttribute node "data-module-id" (:id module))
          (set! (.-title node) (:description module))
          (.append node (el "span" "shelf-piece-name" (:label module)) (el "span" "shelf-piece-pitch" (:description module)))
          (.addEventListener node "click" #(actions/add! (:id module) (count (:segments @es/state))))
          (drag-drop/make-draggable! node {:kind :shelf :module-id (:id module)} "copy" (:label module))
          (.append shelf node))))))

(defn inspector! [state]
  (let [container ($ "inspectorControls")
        selected (es/selected-segment state)]
    (.replaceChildren container)
    (if-not selected
      (do
        (dom/set-text! "inspectorKind" "Separator")
        (dom/set-text! "inspectorTitle" "Whole line")
        (dom/set-text! "inspectorDescription" "What sits between every piece.")
        (dom/set-hidden! "removeSelectedButton" true)
        (.append container
                 (controls/text-control "Separator" (:separator state) actions/set-separator! 12)
                 (controls/colour-control "Separator color" (:separator-color state) actions/set-separator-colour! false)))
      (let [module (es/segment-module state selected)]
        (dom/set-text! "inspectorKind" (:type module))
        (dom/set-text! "inspectorTitle" (:label module))
        (dom/set-text! "inspectorDescription" (:description module))
        (dom/set-hidden! "removeSelectedButton" false)
        (let [field-names (set (map :name (:fields module)))
              command-line? (and (contains? field-names "command") (contains? field-names "args"))]
          (doseq [field (:fields module)]
            (cond
              (and command-line? (= "args" (:name field))) nil
              (and command-line? (= "command" (:name field)))
              (.append container (controls/command-line-control (get-in selected [:config "command"])
                                                                (get-in selected [:config "args"])
                                                                #(actions/set-command-line! (:id selected) %1 %2)))
              :else
              (.append container (controls/control field (:type module) (get-in selected [:config (:name field)])
                                                   #(actions/set-field! (:id selected) (:name field) %))))))))))

(defn capture-positions []
  (into {} (map (fn [node] [(.getAttribute node "data-module-id") (.getBoundingClientRect node)]) (dom/nodes animated-pieces))))

(defn play-flip! [positions]
  (when-not (or (empty? positions) (dom/reduced-motion?))
    (doseq [node (dom/nodes animated-pieces)]
      (when-let [before (get positions (.getAttribute node "data-module-id"))]
        (let [after (.getBoundingClientRect node)
              dx (- (.-left before) (.-left after))
              dy (- (.-top before) (.-top after))]
          (when (or (>= (abs dx) 1) (>= (abs dy) 1))
            (.animate node
                      #js [#js {:transform (str "translate(" dx "px, " dy "px)")} #js {:transform "none"}]
                      #js {:duration 260 :easing "cubic-bezier(0.2, 0.85, 0.25, 1)"})))))))

(defn font-size! [state]
  (.setProperty (.-style ($ "lineScreen")) "--screen-font-px" (str (:font-px state)))
  (dom/set-text! "fontSizeValue" (str (:font-px state) "px")))

(defn all! [state]
  (let [positions (capture-positions)]
    (shelf! state)
    (font-size! state)
    (line! state)
    (inspector! state)
    (play-flip! positions)))

(defn changes! [old new]
  (let [positions (when (not= (es/shelf-key old) (es/shelf-key new)) (capture-positions))]
    (cond
      (not= (es/line-key old) (es/line-key new)) (line! new)
      (not= (:selected-id old) (:selected-id new)) (selection! new))
    (when positions
      (shelf! new)
      (play-flip! positions))
    (when (not= (:font-px old) (:font-px new))
      (font-size! new))
    (when (not= (es/inspector-key old) (es/inspector-key new))
      (inspector! new))))
