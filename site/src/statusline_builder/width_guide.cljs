(ns statusline-builder.width-guide
  (:require [statusline-builder.actions :as actions]
            [statusline-builder.dom :refer [$]]
            [statusline-builder.editor-state :as es]
            [statusline-builder.layout :as layout]))

(def ^:private screen-padding 24)

(defn- columns-at [client-x]
  (let [screen ($ "lineScreen")
        canvas-width (.-width (.getBoundingClientRect ($ "lineCanvas")))
        column-px (/ canvas-width (- (:terminal-width @es/state) 2))
        x (- (+ client-x (.-scrollLeft screen)) (.-left (.getBoundingClientRect screen)) screen-padding)]
    (layout/snap-columns (/ x column-px))))

(defn- start! [event]
  (let [guide (.-currentTarget event)]
    (.preventDefault event)
    (.setPointerCapture guide (.-pointerId event))
    (.add (.-classList guide) "is-dragging")))

(defn- drag! [event]
  (let [guide (.-currentTarget event)]

    (when (.hasPointerCapture guide (.-pointerId event))
      (let [columns (columns-at (.-clientX event))]

        (when (not= columns (:terminal-width @es/state))
          (actions/set-terminal-width! columns))))))

(defn- stop! [event]
  (let [guide (.-currentTarget event)]

    (when (.hasPointerCapture guide (.-pointerId event))
      (.releasePointerCapture guide (.-pointerId event)))
    (.remove (.-classList guide) "is-dragging")))

(defn bind! []
  (let [guide ($ "widthGuide")]
    (.addEventListener guide "pointerdown" start!)
    (.addEventListener guide "pointermove" drag!)
    (.addEventListener guide "pointerup" stop!)
    (.addEventListener guide "pointercancel" stop!)))
