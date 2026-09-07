(ns statusline-builder.actions
  (:require [statusline-builder.dom :as dom]
            [statusline-builder.editor-state :as es]))

(defn- label [state id]
  (:label (es/segment-module state (es/segment state id))))

(defn- change! [transition]
  (let [before @es/state
        after (swap! es/state transition)]
    (when-not (identical? before after)
      [before after])))

(defn add! [module-id index]
  (when-let [[_ after] (change! #(es/add-segment % module-id index))]
    (let [id (:selected-id after)]
      (dom/announce! (str (label after id) " put on the line at position " (inc (es/segment-index after id)) "."))
      (dom/focus-piece! id))))

(defn remove! [id]
  (when-let [[before _] (change! #(es/remove-segment % id))]
    (dom/announce! (str (label before id) " taken off the line."))))

(defn move! [id delta]
  (when-let [[_ after] (change! #(es/move-segment % id delta))]
    (dom/announce! (str (label after id) " moved to position " (inc (es/segment-index after id)) "."))
    (dom/focus-piece! id)))

(defn move-to! [id index]
  (when-let [[_ after] (change! #(es/move-segment-to % id index))]
    (dom/announce! (str (label after id) " moved to position " (inc (es/segment-index after id)) "."))
    (dom/focus-piece! id)))

(defn select! [target]
  (swap! es/state es/select target))

(defn set-field! [id field value]
  (swap! es/state es/set-field id field value))

(defn set-separator! [value]
  (swap! es/state assoc :separator value))

(defn set-separator-colour! [value]
  (swap! es/state assoc :separator-color value))

(defn set-scenario! [id]
  (swap! es/state assoc :scenario-id id))

(defn set-terminal-width! [columns]
  (swap! es/state assoc :terminal-width columns))
