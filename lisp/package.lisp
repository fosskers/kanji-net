(defpackage kanjinet
  (:use :cl)
  (:local-nicknames
   (#:f #:filepaths)
   (#:g #:simple-graph)
   (#:p #:parcom)
   (#:t #:transducers))
  (:export #:main))

(in-package :kanjinet)

;; --- Blessed Functions --- ;;

(defmacro fn (name type)
  "A shorthand for declaiming function types."
  `(declaim (ftype ,type ,name)))

(deftype -> (a b &rest args)
  "A shorthand for function types."
  (if (null args)
      `(function (,a) ,b)
      (let ((argz (butlast args))
            (res (car (last args))))
        `(function (,a ,b ,@argz) ,res))))

(defmacro hmap (&rest items)
  "A short-hand for defining key-value pairs."
  (let ((ht (gensym "MAP-NAME")))
    `(let ((,ht (make-hash-table :test #'equal)))
       ,@(t:transduce (t:comp (t:segment 2)
                              (t:map (lambda (list)
                                       (let ((1st (car list))
                                             (2nd (cadr list)))
                                         `(setf (gethash ,1st ,ht) ,2nd)))))
                      #'t:cons items)
       ,ht)))

(declaim (ftype (function (string string &key (:from fixnum)) boolean) string-starts-with?))
(defun string-starts-with? (s prefix &key (from 0))
  (string= prefix s :start2 from :end2 (min (+ from (length prefix))
                                            (length s))))

(fn read-string (-> (or string pathname) (simple-array character (*))))
(defun read-string (path)
  "Read the contents of a file into a string."
  (with-open-file (stream path :direction :input :element-type 'character)
    (with-output-to-string (out)
      (loop :for c := (read-char stream nil :eof)
            :until (eq c :eof)
            :do (write-char c out)))))
