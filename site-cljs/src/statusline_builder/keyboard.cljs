(ns statusline-builder.keyboard
  (:require [statusline-builder.actions :as actions]
            [statusline-builder.dom :as dom]
            [statusline-builder.editor-state :as es]))

(defn- move-along-line! [state id direction]
  (let [visible (mapv dom/segment-id (dom/line-piece-nodes))
        position (dom/index-of visible id)]
    (if (nil? position)
      (actions/move! id direction)
      (when-let [neighbour (es/segment-index state (get visible (+ position direction)))]
        (actions/move-to! id (if (pos? direction) (inc neighbour) neighbour))))))

(defn- editing? [target]
  (or (nil? target)
      (not (instance? js/Element target))
      (some? (.closest target "input, textarea, select, dialog, [contenteditable]"))))

(defn- handle-key! [event]
  (when-not (or (.-defaultPrevented event) (.-altKey event) (.-ctrlKey event) (.-metaKey event) (editing? (.-target event)))
    (let [state @es/state]
      (when-let [selected (es/selected-segment state)]
        (case (.-key event)
          "ArrowLeft" (do (.preventDefault event) (move-along-line! state (:id selected) -1))
          "ArrowRight" (do (.preventDefault event) (move-along-line! state (:id selected) 1))
          ("Delete" "Backspace") (do (.preventDefault event) (actions/remove! (:id selected)))
          nil)))))

(defn bind! []
  (.addEventListener js/document "keydown" handle-key!))
