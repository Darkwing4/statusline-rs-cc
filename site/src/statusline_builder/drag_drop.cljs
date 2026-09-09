(ns statusline-builder.drag-drop
  (:require [statusline-builder.actions :as actions]
            [statusline-builder.dom :as dom :refer [$]]
            [statusline-builder.editor-state :as es]
            [statusline-builder.layout :as layout]))

(defonce ^:private payload (atom nil))

(defn clear-indicators! []
  (.remove (.-classList ($ "lineCanvas")) "is-dropzone")
  (doseq [node (array-seq (.querySelectorAll ($ "lineCanvas") ".drop-before, .drop-after"))]
    (.remove (.-classList node) "drop-before" "drop-after")))

(defn make-draggable! [node data effect label]
  (set! (.-draggable node) true)
  (.addEventListener node "dragstart"
                     (fn [event]
                       (reset! payload data)
                       (.add (.-classList node) "is-dragging")
                       (set! (.-effectAllowed (.-dataTransfer event)) effect)
                       (.setData (.-dataTransfer event) "text/plain" label)))
  (.addEventListener node "dragend"
                     (fn [_]
                       (reset! payload nil)
                       (.remove (.-classList node) "is-dragging")
                       (clear-indicators!))))

(defn- visible-piece-nodes []
  (let [dragged (when (= :line (:kind @payload)) (:id @payload))]
    (vec (remove #(= dragged (dom/segment-id %)) (dom/line-piece-nodes)))))

(defn- rect [node]
  (let [box (.getBoundingClientRect node)]
    {:left (.-left box) :right (.-right box) :top (.-top box) :bottom (.-bottom box)}))

(defn- drop-slot [event]
  (layout/nearest-drop-slot (map (fn [node] {:node node :rect (rect node)}) (visible-piece-nodes))
                            (.-clientX event)
                            (.-clientY event)))

(defn- drop-index [event state]
  (if-let [{:keys [node before]} (drop-slot event)]
    (let [visible (visible-piece-nodes)
          left (if before (get visible (dec (.indexOf visible node))) node)]
      (if left
        (inc (es/segment-index state (dom/segment-id left)))
        0))
    (count (:segments state))))

(defn- drag-over! [event]
  (when-let [data @payload]
    (.preventDefault event)
    (set! (.-dropEffect (.-dataTransfer event)) (if (= :shelf (:kind data)) "copy" "move"))
    (clear-indicators!)
    (.add (.-classList ($ "lineCanvas")) "is-dropzone")
    (when-let [{:keys [node before]} (drop-slot event)]
      (.add (.-classList node) (if before "drop-before" "drop-after")))))

(defn- drop! [event]
  (when-let [data @payload]
    (.preventDefault event)
    (let [index (drop-index event @es/state)]
      (reset! payload nil)
      (clear-indicators!)
      (if (= :shelf (:kind data))
        (actions/add! (:module-id data) index)
        (actions/move-to! (:id data) index)))))

(defn bind! []
  (.addEventListener js/document "dragover" drag-over!)
  (.addEventListener js/document "dragleave" #(when (nil? (.-relatedTarget %)) (clear-indicators!)))
  (.addEventListener js/document "drop" drop!))
