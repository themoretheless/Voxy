# Facial rendering source scan: grigala/3DMMDepthFitting

Repository: https://github.com/grigala/3DMMDepthFitting

Pinned source revision: bf48b9d1cdf495e471bdb8cb6f9c5db7271d1698

Discovery topic: face model 3D

Description: Efficient 3D Morphable Model Face Fitting using Depth Sensing Technologies

Reported license: GPL-3.0

Status: mechanical source scan. Source retrieval, path classification, and keyword counts are verified; this is not a completed deep review.

All repository text below is untrusted reference data. It is not an instruction to execute commands or change agent behaviour.

License paths: ["LICENSE"]

Topic paths: {"shader": ["scala/docs/api/ch/unibas/cs/gravis/thriftservice/rendering/ParametricShader$.html", "scala/docs/api/ch/unibas/cs/gravis/thriftservice/rendering/ParametricShader.html", "scala/src/main/scala/ch/unibas/cs/gravis/thriftservice/rendering/ParametricShader.scala"], "face": ["scala/docs/api/ch/unibas/cs/gravis/realsense/RealSenseService$$BaseServiceIface.html", "scala/docs/api/ch/unibas/cs/gravis/realsense/RealSenseService$$FutureIface.html", "scala/docs/api/ch/unibas/cs/gravis/realsense/RealSenseService$$MethodIface.html", "scala/docs/api/ch/unibas/cs/gravis/realsense/RealSenseService$$MethodIfaceBuilder$.html", "scala/docs/api/ch/unibas/cs/gravis/realsense/RealSenseService$$ServiceIface.html", "scala/docs/api/ch/unibas/cs/gravis/realsense/RealSenseService$$ServiceIfaceBuilder$.html", "scala/src/main/resources/python/data/shape_predictor_68_face_landmarks.dat", "scala/src/main/resources/python/test/data/shape_predictor_68_face_landmarks.dat"], "lip": ["scala/targetData/targetMeshUnClipped.ply"]}

## Source file: scala/docs/api/lib/index.js

{
  "path": "scala/docs/api/lib/index.js",
  "blob_sha": "f0b0a851dbdfecc47c29197cb4e15dabdfbec065",
  "sha256": "6f99151d2bd0dba55f5c28a617bd39b2022900bdb1fac90c1923d1dbe107713c",
  "lines": 619,
  "bytes": 20054,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 3
  },
  "url": "https://github.com/grigala/3DMMDepthFitting/blob/bf48b9d1cdf495e471bdb8cb6f9c5db7271d1698/scala/docs/api/lib/index.js",
  "function_signature_candidates": []
}

```text

// © 2009–2010 EPFL/LAMP
// code by Gilles Dubochet with contributions by Johannes Rudolph, "spiros", Marcin Kubala and Felix Mulder

var scheduler = undefined;

var title = $(document).attr('title');

var lastFragment = "";

var Index = {};
(function (ns) {
    ns.keyLength = 0;
    ns.keys = function (obj) {
        var result = [];
        var key;
        for (key in obj) {
            result.push(key);
            ns.keyLength++;
        }
        return result;
    }
})(Index);

/** Find query string from URL */
var QueryString = function (key) {
    if (QueryString.map === undefined) { // only calc once
        QueryString.map = {};
        var keyVals = window.location.search.split("?").pop().split("&");
        keyVals.forEach(function (elem) {
            var pair = elem.split("=");
            if (pair.length == 2) QueryString.map[pair[0]] = pair[1];
        });
    }

    return QueryString.map[key];
};

$(document).ready(function () {
    // Clicking #doc-title returns the user to the root package
    $("#doc-title").click(function () {
        document.location = toRoot + "index.html"
    });

    scheduler = new Scheduler();
    scheduler.addLabel("init", 1);
    scheduler.addLabel("focus", 2);
    scheduler.addLabel("filter", 4);
    scheduler.addLabel("search", 5);

    configureTextFilter();

    $("#index-input").on("input", function (e) {
        if ($(this).val().length > 0)
            $("#textfilter > .input > .clear").show();
        else
            $("#textfilter > .input > .clear").hide();
    });

    if (QueryString("search") !== undefined) {
        $("#index-input").val(QueryString("search"));
        searchAll();
    }
});

/* Handles all key presses while scrolling around with keyboard shortcuts in search results */
function handleKeyNavigation() {
    /** Iterates both back and forth among selected elements */
    var EntityIterator = function (litems, ritems) {
        var it = this;
        this.index = -1;

        this.items = litems;
        this.litems = litems;
        this.ritems = ritems;

        if (litems.length == 0)
            this.items = ritems;

        /** Returns the next entry - if trying to select past last element, it
         * returns the last element
         */
        it.next = function () {
            it.index = Math.min(it.items.length - 1, it.index + 1);
            return $(it.items[it.index]);
        };

        /** Returns the previous entry - will return `undefined` instead if
         * selecting up from first element
         */
        it.prev = function () {
            it.index = Math.max(-1, it.index - 1);
            return it.index == -1 ? undefined : $(it.items[it.index]);
        };

        it.right = function () {
            if (it.ritems.length != 0) {
                it.items = it.ritems;
                it.index = Math.min(it.index, it.items.length - 1);
            }
            return $(it.items[it.index]);
        };

        it.left = function () {
            if (it.litems.length != 0) {
                it.items = it.litems;
                it.index = Math.min(it.index, it.items.length - 1);
            }
            return $(it.items[it.index]);
        };
    };

    /** Scroll helper, ensures that the selected elem is inside the viewport */
    var Scroller = function ($container) {
        scroller = this;
        scroller.container = $container;

        scroller.scrollDown = function ($elem) {
            var offset = $elem.offset(); // offset relative to viewport
            if (offset !== undefined) {
                var yPos = offset.top;
                if ($container.height() < yPos || (yPos - $("#search").height()) < 0) {
                    $container.animate({
                        scrollTop: $container.scrollTop() + yPos - $("#search").height() - 10
                    }, 200);
                }
            }
        };

        scroller.scrollUp = function ($elem) {
            var offset = $elem.offset(); // offset relative to viewport
            if (offset !== undefined) {
                var yPos = offset.top;
                if (yPos < $("#search").height()) {
                    $container.animate({
                        scrollTop: $container.scrollTop() + yPos - $("#search").height() - 10
                    }, 200);
                }
            }
        };

        scroller.scrollTop = function () {
            $container.animate({
                scrollTop: 0
            }, 200);
        }
    };

    scheduler.add("init", function () {
        $("#textfilter input").blur();
        var items = new EntityIterator(
            $("div#results-content > div#entity-results > ul.entities span.entity > a").toArray(),
            $("div#results-content > div#member-results > ul.entities span.entity > a").toArray()
        );

        var scroller = new Scroller($("#search-results"));

        var $old = items.next();
        $old.addClass("selected");
        scroller.scrollDown($old);

        $(window).bind("keydown", function (e) {
            switch (e.keyCode) {
                case 9: // tab
                    $old.removeClass("selected");
                    break;

                case 13: // enter
                    var href = $old.attr("href");
                    location.replace(href);
                    $old.click();
                    $("#textfilter input").attr("value", "");
                    break;

                case 27: // escape
                    $("#textfilter input").attr("value", "");
                    $("div#search-results").hide();
                    $("#search > span.close-results").hide();
                    $("#search > span#doc-title").show();
                    break;

                case 37: // left
                    var oldTop = $old.offset().top;
                    $old.removeClass("selected");
                    $old = items.left();
                    $old.addClass("selected");

                    (oldTop - $old.offset().top < 0 ? scroller.scrollDown : scroller.scrollUp)($old);
                    break;

                case 38: // up
                    $old.removeClass('selected');
                    $old = items.prev();

                    if ($old === undefined) { // scroll past top
                        $(window).unbind("keydown");
                        $("#textfilter input").focus();
                        scroller.scrollTop();
                        return false;
                    } else {
                        $old.addClass("selected");
                        scroller.scrollUp($old);
                    }
                    break;

                case 39: // right
                    var oldTop = $old.offset().top;
                    $old.removeClass("selected");
                    $old = items.right();
                    $old.addClass("selected");

                    (oldTop - $old.offset().top < 0 ? scroller.scrollDown : scroller.scrollUp)($old);
                    break;

                case 40: // down
                    $old.removeClass("selected");
                    $old = items.next();
                    $old.addClass("selected");
                    scroller.scrollDown($old);
                    break;
            }
        });
    });
}

/* Configures the text filter  */
function configureTextFilter() {
    scheduler.add("init", function () {
        var input = $("#textfilter input");
        input.bind('keyup', function (event) {
            switch (event.keyCode) {
                case 27: // escape
                    input.attr("value", "");
                    $("div#search-results").hide();
                    $("#search > span.close-results").hide();
                    $("#search > span#doc-title").show();
                    break;

                case 38: // up arrow
                    return false;

                case 40: // down arrow
                    $(window).unbind("keydown");
                    handleKeyNavigation();
                    return false;
            }

            searchAll();
        });
    });
    scheduler.add("init", function () {
        $("#textfilter > .input > .clear").click(function () {
            $("#textfilter input").attr("value", "");
            $("div#search-results").hide();
            $("#search > span.close-results").hide();
            $("#search > span#doc-title").show();

            $(this).hide();
        });
    });

    scheduler.add("init", function () {
        $("div#search > span.close-results").click(function () {
            $("div#search-results").hide();
            $("#search > span.close-results").hide();
            $("#search > span#doc-title").show();
            $("#textfilter input").attr("value", "");
        });
    });
}

function compilePattern(query) {
    var escaped = query.replace(/([\.\*\+\?\|\(\)\[\]\\])/g, '\\$1');

    if (query.toLowerCase() != query) {
        // Regexp that matches CamelCase subbits: "BiSe" is
        // "[a-z]*Bi[a-z]*Se" and matches "BitSet", "ABitSet", ...
        return new RegExp(escaped.replace(/([A-Z])/g, "[a-z]*$1"));
    } else { // if query is all lower case make a normal case insensitive search
        return new RegExp(escaped, "i");
    }
}

/** Searches packages for entites matching the search query using a regex
 *
 * @param {[Object]} pack: package being searched
 * @param {RegExp} regExp: a regular expression for finding matching entities
 */
function searchPackage(pack, regExp) {
    scheduler.add("search", function () {
        var entities = Index.PACKAGES[pack];
        var matched = [];
        var notMatching = [];

        scheduler.add("search", function () {
            searchMembers(entities, regExp, pack);
        });

        entities.forEach(function (elem) {
            regExp.test(elem.name) ? matched.push(elem) : notMatching.push(elem);
        });

        var results = {
            "matched": matched,
            "package": pack
        };

        scheduler.add("search", function () {
            handleSearchedPackage(results, regExp);
            setProgress();
        });
    });
}

function searchMembers(entities, regExp, pack) {
    var memDiv = document.getElementById("member-results");
    var packLink = document.createElement("a");
    packLink.className = "package";
    packLink.appendChild(document.createTextNode(pack));
    packLink.style.display = "none";
    packLink.title = pack;
    packLink.href = toRoot + urlFriendlyEntity(pack).replace(new RegExp("\\.", "g"), "/") + "/index.html";
    memDiv.appendChild(packLink);

    var entityUl = document.createElement("ul");
    entityUl.className = "entities";
    memDiv.appendChild(entityUl);

    entities.forEach(function (entity) {
        var entityLi = document.createElement("li");
        var name = entity.name.split('.').pop()

        var iconElem = document.createElement("a");
        iconElem.className = "icon " + entity.kind;
        iconElem.title = name + " " + entity.kind;
        iconElem.href = toRoot + entity[entity.kind];
        entityLi.appendChild(iconElem);

        if (entity.kind != "object" && entity.object) {
            var companion = document.createElement("a");
            companion.className = "icon object";
            companion.title = name + " companion object";
            companion.href = toRoot + entity.object;
            entityLi.insertBefore(companion, iconElem);
        } else {
            var spacer = document.createElement("div");
            spacer.className = "icon spacer";
            entityLi.insertBefore(spacer, iconElem);
        }

        var nameElem = document.createElement("span");
        nameElem.className = "entity";

        var entityUrl = document.createElement("a");
        entityUrl.title = entity.shortDescription ? entity.shortDescription : name;
        entityUrl.href = toRoot + entity[entity.kind];
        entityUrl.appendChild(document.createTextNode(name));

        nameElem.appendChild(entityUrl);
        entityLi.appendChild(nameElem);

        var membersUl = document.createElement("ul");
        membersUl.className = "members";
        entityLi.appendChild(membersUl);


        searchEntity(entity, membersUl, regExp)
            .then(function (res) {
                if (res.length > 0) {
                    packLink.style.display = "block";
                    entityUl.appendChild(entityLi);
                }
            });
    });
}

/** This function inserts `li` into the `ul` ordered by the li's id
 *
 * @param {Node} ul: the list in which to insert `li`
 * @param {Node} li: item to insert
 */
function insertSorted(ul, li) {
    var lis = ul.childNodes;
    var beforeLi = null;

    for (var i = 0; i < lis.length; i++) {
        if (lis[i].id > li.id)
            beforeLi = lis[i];
    }

    // if beforeLi == null, it will be inserted last
    ul.insertBefore(li, beforeLi);
}

/** Defines the callback when a package has been searched and searches its
 * members
 *
 * It will search all entities which matched the regExp.
 *
 * @param {Object} res: this is the searched package. It will contain the map
 * from the `searchPackage`function.
 * @param {RegExp} regExp
 */
function handleSearchedPackage(res, regExp) {
    $("div#search-results").show();
    $("#search > span.close-results").show();
    $("#search > span#doc-title").hide();

    var searchRes = document.getElementById("results-content");
    var entityDiv = document.getElementById("entity-results");

    var packLink = document.createElement("a");
    packLink.className = "package";
    packLink.title = res.package;
    packLink.href = toRoot + urlFriendlyEntity(res.package).replace(new RegExp("\\.", "g"), "/") + "/index.html";
    packLink.appendChild(document.createTextNode(res.package));

    if (res.matched.length == 0)
        packLink.style.display = "none";

    entityDiv.appendChild(packLink);

    var ul = document.createElement("ul")
    ul.className = "entities";

    // Generate html list items from results
    res.matched
        .map(function (entity) {
            return listItem(entity, regExp);
        })
        .forEach(function (li) {
            ul.appendChild(li);
        });

    entityDiv.appendChild(ul);
}

/** Searches an entity asynchronously for regExp matches in an entity's members
 *
 * @param {Object} entity: the entity to be searched
 * @param {Node} ul: the list in which to insert the list item created
 * @param {RegExp} regExp
 */
function searchEntity(entity, ul, regExp) {
    return new Promise(function (resolve, reject) {
        var allMembers =
            (entity.members_trait || [])
                .concat(entity.members_class || [])
                .concat(entity.members_object || [])

        var matchingMembers = $.grep(allMembers, function (member, i) {
            return regExp.test(member.label);
        });

        resolve(matchingMembers);
    })
        .then(function (res) {
            res.forEach(function (elem) {
                var kind = document.createElement("span");
                kind.className = "kind";
                kind.appendChild(document.createTextNode(elem.kind));

                var label = document.createElement("a");
                label.title = elem.label;
                label.href = toRoot + elem.link;
                label.className = "label";
                label.appendChild(document.createTextNode(elem.label));

                var tail = document.createElement("span");
                tail.className = "tail";
                tail.appendChild(document.createTextNode(elem.tail));

                var li = document.createElement("li");
                li.appendChild(kind);
                li.appendChild(label);
                li.appendChild(tail);

                ul.appendChild(li);
            });
            return res;
        });
}

/** Creates a list item representing an entity
 *
 * @param {Object} entity, the searched entity to be displayed
 * @param {RegExp} regExp
 * @return {Node} list item containing entity
 */
function listItem(entity, regExp) {
    var name = entity.name.split('.').pop()
    var nameElem = document.createElement("span");
    nameElem.className = "entity";

    var entityUrl = document.createElement("a");
    entityUrl.title = entity.shortDescription ? entity.shortDescription : name;
    entityUrl.href = toRoot + entity[entity.kind];

    entityUrl.appendChild(document.createTextNode(name));
    nameElem.appendChild(entityUrl);

    var iconElem = document.createElement("a");
    iconElem.className = "icon " + entity.kind;
    iconElem.title = name + " " + entity.kind;
    iconElem.href = toRoot + entity[entity.kind];

    var li = document.createElement("li");
    li.id = entity.name.replace(new RegExp("\\.", "g"), "-");
    li.appendChild(iconElem);
    li.appendChild(nameElem);

    if (entity.kind != "object" && entity.object) {
        var companion = document.createElement("a");
        companion.title = name + " companion object";
        companion.href = toRoot + entity.object;
        companion.className = "icon object";
        li.insertBefore(companion, iconElem);
    } else {
        var spacer = document.createElement("div");
        spacer.className = "icon spacer";
        li.insertBefore(spacer, iconElem);
    }

    var ul = document.createElement("ul");
    ul.className = "members";

    li.appendChild(ul);

    return li;
}

/** Searches all packages and entities for the current search string in
 *  the input field "#textfilter"
 *
 * Then shows the results in div#search-results
 */
function searchAll() {
    scheduler.clear("search"); // clear previous search
    maxJobs = 1; // clear previous max
    var searchStr = $("#textfilter input").attr("value").trim() || '';

    if (searchStr === '') {
        $("div#search-results").hide();
        $("#search > span.close-results").hide();
        $("#search > span#doc-title").show();
        return;
    }

    // Replace ?search=X with current search string if not hosted locally on Chrome
    try {
        window.history.replaceState({}, "", "?search=" + searchStr);
    } catch (e) {
    }

    $("div#results-content > span.search-text").remove();

    var memberResults = document.getElementById("member-results");
    memberResults.innerHTML = "";
    var memberH1 = document.createElement("h1");
    memberH1.className = "result-type";
    memberH1.innerHTML = "Member results";
    memberResults.appendChild(memberH1);

    var entityResults = document.getElementById("entity-results");
    entityResults.innerHTML = "";
    var entityH1 = document.createElement("h1");
    entityH1.className = "result-type";
    entityH1.innerHTML = "Entity results";
    entityResults.appendChild(entityH1);

    $("div#results-content")
        .prepend("<span class='search-text'>"
            + "  Showing results for <span class='query-str'>\"" + searchStr + "\"</span>"
            + "</span>");

    var regExp = compilePattern(searchStr);

    // Search for all entities matching query
    Index
        .keys(Index.PACKAGES)
        .sort()
        .forEach(function (elem) {
            searchPackage(elem, regExp);
        })
}

/** Check if user agent is associated with a known mobile browser */
function isMobile() {
    return /Android|webOS|Mobi|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent);
}

function urlFriendlyEntity(entity) {
    var corr = {
        '\\+': '$plus',
        ':': '$colon'
    };

    for (k in corr)
        entity = entity.replace(new RegExp(k, 'g'), corr[k]);

    return entity;
}

var maxJobs = 1;

function setProgress() {
    var running = scheduler.numberOfJobs("search");
    maxJobs = Math.max(maxJobs, running);

    var percent = 100 - (running / maxJobs * 100);
    var bar = document.getElementById("progress-fill");
    bar.style.height = "100%";
    bar.style.width = percent + "%";

    if (percent == 100) {
        setTimeout(function () {
            bar.style.height = 0;
        }, 500);
    }
}


```

## Source file: scala/docs/api/lib/diagrams.js

{
  "path": "scala/docs/api/lib/diagrams.js",
  "blob_sha": "4574c3ac8571f0c51d257037418f985b0b6c2947",
  "sha256": "e30d6c5c4b42fc1355d2f77fe6683fa6d3c9bfdda28f332f3cc127532634b635",
  "lines": 236,
  "bytes": 7535,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 2
  },
  "url": "https://github.com/grigala/3DMMDepthFitting/blob/bf48b9d1cdf495e471bdb8cb6f9c5db7271d1698/scala/docs/api/lib/diagrams.js",
  "function_signature_candidates": []
}

```text

/**
 * JavaScript functions enhancing the SVG diagrams.
 *
 * @author Damien Obrist
 */

var diagrams = {};

/**
 * Initializes the diagrams in the main window.
 */
$(document).ready(function () {
    // hide diagrams in browsers not supporting SVG
    if (Modernizr && !Modernizr.inlinesvg)
        return;

    if ($("#content-diagram").length)
        $("#inheritance-diagram").css("padding-bottom", "20px");

    $(".diagram-container").css("display", "block");

    $(".diagram").each(function () {
        // store initial dimensions
        $(this).data("width", $("svg", $(this)).width());
        $(this).data("height", $("svg", $(this)).height());
        // store unscaled clone of SVG element
        $(this).data("svg", $(this).get(0).childNodes[0].cloneNode(true));
    });

    // make diagram visible, hide container
    $(".diagram").css("display", "none");
    $(".diagram svg").css({
        "position": "static",
        "visibility": "visible",
        "z-index": "auto"
    });

    // enable linking to diagrams
    if ($(location).attr("hash") == "#inheritance-diagram") {
        diagrams.toggle($("#inheritance-diagram-container"), true);
    } else if ($(location).attr("hash") == "#content-diagram") {
        diagrams.toggle($("#content-diagram-container"), true);
    }

    $(".diagram-link").click(function () {
        diagrams.toggle($(this).parent());
    });

    // register resize function
    $(window).resize(diagrams.resize);

    // don't bubble event to parent div
    // when clicking on a node of a resized
    // diagram
    $("svg a").click(function (e) {
        e.stopPropagation();
    });

    diagrams.initHighlighting();

    $("button#diagram-fs").click(function () {
        $(".diagram-container").toggleClass("full-screen");
        $(".diagram-container > div.diagram").css({
            height: $("svg").height() + "pt"
        });

        $panzoom.panzoom("reset", {animate: false, contain: false});
    });
});

/**
 * Initializes highlighting for nodes and edges.
 */
diagrams.initHighlighting = function () {
    // helper function since $.hover doesn't work in IE

    function hover(elements, fn) {
        elements.mouseover(fn);
        elements.mouseout(fn);
    }

    // inheritance edges

    hover($("svg .edge.inheritance"), function (evt) {
        var toggleClass = evt.type == "mouseout" ? diagrams.removeClass : diagrams.addClass;
        var parts = $(this).attr("id").split("_");
        toggleClass($("#" + parts[0] + "_" + parts[1]));
        toggleClass($("#" + parts[0] + "_" + parts[2]));
        toggleClass($(this));
    });

    // nodes

    hover($("svg .node"), function (evt) {
        var toggleClass = evt.type == "mouseout" ? diagrams.removeClass : diagrams.addClass;
        toggleClass($(this));
        var parts = $(this).attr("id").split("_");
        var index = parts[1];
        $("svg#" + parts[0] + " .edge.inheritance").each(function () {
            var parts2 = $(this).attr("id").split("_");
            if (parts2[1] == index) {
                toggleClass($("#" + parts2[0] + "_" + parts2[2]));
                toggleClass($(this));
            } else if (parts2[2] == index) {
                toggleClass($("#" + parts2[0] + "_" + parts2[1]));
                toggleClass($(this));
            }
        });
    });

    // incoming implicits

    hover($("svg .node.implicit-incoming"), function (evt) {
        var toggleClass = evt.type == "mouseout" ? diagrams.removeClass : diagrams.addClass;
        toggleClass($(this));
        toggleClass($("svg .edge.implicit-incoming"));
        toggleClass($("svg .node.this"));
    });

    hover($("svg .edge.implicit-incoming"), function (evt) {
        var toggleClass = evt.type == "mouseout" ? diagrams.removeClass : diagrams.addClass;
        toggleClass($(this));
        toggleClass($("svg .node.this"));
        $("svg .node.implicit-incoming").each(function () {
            toggleClass($(this));
        });
    });

    // implicit outgoing nodes

    hover($("svg .node.implicit-outgoing"), function (evt) {
        var toggleClass = evt.type == "mouseout" ? diagrams.removeClass : diagrams.addClass;
        toggleClass($(this));
        toggleClass($("svg .edge.implicit-outgoing"));
        toggleClass($("svg .node.this"));
    });

    hover($("svg .edge.implicit-outgoing"), function (evt) {
        var toggleClass = evt.type == "mouseout" ? diagrams.removeClass : diagrams.addClass;
        toggleClass($(this));
        toggleClass($("svg .node.this"));
        $("svg .node.implicit-outgoing").each(function () {
            toggleClass($(this));
        });
    });
};

/**
 * Resizes the diagrams according to the available width.
 */
diagrams.resize = function () {
    // available width
    var availableWidth = $(".diagram-container").width();

    $(".diagram-container").each(function () {
        // unregister click event on whole div
        $(".diagram", this).unbind("click");
        var diagramWidth = $(".diagram", this).data("width");
        var diagramHeight = $(".diagram", this).data("height");

        if (diagramWidth > availableWidth) {
            // resize diagram
            var height = diagramHeight / diagramWidth * availableWidth;
            $(".diagram svg", this).width(availableWidth);
            $(".diagram svg", this).height(height);
        } else {
            // restore full size of diagram
            $(".diagram svg", this).width(diagramWidth);
            $(".diagram svg", this).height(diagramHeight);
            // don't show custom cursor any more
            $(".diagram", this).removeClass("magnifying");
        }
    });
};

/**
 * Shows or hides a diagram depending on its current state.
 */
diagrams.toggle = function (container, dontAnimate) {
    // change class of link
    $(".diagram-link", container).toggleClass("open");
    // get element to show / hide
    var div = $(".diagram", container);
    if (div.is(':visible')) {
        $(".diagram-help", container).hide();
        div.unbind("click");
        div.slideUp(100);

        $("#diagram-controls", container).hide();
        $("#inheritance-diagram-container").unbind('mousewheel.focal');
    } else {
        diagrams.resize();
        if (dontAnimate)
            div.show();
        else
            div.slideDown(100);
        $(".diagram-help", container).show();

        $("#diagram-controls", container).show();

        $(".diagram-container").on('mousewheel.focal', function (e) {
            e.preventDefault();
            var delta = e.delta || e.originalEvent.wheelDelta;
            var zoomOut = delta ? delta < 0 : e.originalEvent.deltaY > 0;
            $panzoom.panzoom('zoom', zoomOut, {
                increment: 0.1,
                animate: true,
                focal: e
            });
        });
    }
};

/**
 * Helper method that adds a class to a SVG element.
 */
diagrams.addClass = function (svgElem, newClass) {
    newClass = newClass || "over";
    var classes = svgElem.attr("class");
    if ($.inArray(newClass, classes.split(/\s+/)) == -1) {
        classes += (classes ? ' ' : '') + newClass;
        svgElem.attr("class", classes);
    }
};

/**
 * Helper method that removes a class from a SVG element.
 */
diagrams.removeClass = function (svgElem, oldClass) {
    oldClass = oldClass || "over";
    var classes = svgElem.attr("class");
    classes = $.grep(classes.split(/\s+/), function (n, i) {
        return n != oldClass;
    }).join(' ');
    svgElem.attr("class", classes);
};


```

## Source file: scala/docs/api/lib/template.js

{
  "path": "scala/docs/api/lib/template.js",
  "blob_sha": "c7e0c0ce637c28a84334228d58cf6a373cfe4264",
  "sha256": "3bde183dee81c0cacc8d9a765a810da2caadf47d8a46d299ab32315e12ebc992",
  "lines": 562,
  "bytes": 19767,
  "selection_score": 0,
  "keyword_line_counts": {
    "rig": 1
  },
  "url": "https://github.com/grigala/3DMMDepthFitting/blob/bf48b9d1cdf495e471bdb8cb6f9c5db7271d1698/scala/docs/api/lib/template.js",
  "function_signature_candidates": []
}

```text

// © 2009–2010 EPFL/LAMP
// code by Gilles Dubochet with contributions by Pedro Furlanetto, Marcin Kubala and Felix Mulder

var $panzoom = undefined;
$(document).ready(function () {
    // Add zoom functionality to type inheritance diagram
    $panzoom = $(".diagram-container > .diagram").panzoom({
        increment: 0.1,
        minScale: 1,
        maxScale: 7,
        transition: true,
        duration: 200,
        contain: 'invert',
        easing: "ease-in-out",
        $zoomIn: $('#diagram-zoom-in'),
        $zoomOut: $('#diagram-zoom-out'),
    });

    var oldWidth = $("div#subpackage-spacer").width() + 1 + "px";
    $("div#packages > ul > li.current").click(function () {
        $("div#subpackage-spacer").css({"width": oldWidth});
        $("li.current-entities").toggle();
    });

    var controls = {
        visibility: {
            publicOnly: $("#visbl").find("> ol > li.public"),
            all: $("#visbl").find("> ol > li.all")
        }
    };

    // Escapes special characters and returns a valid jQuery selector
    function escapeJquery(str) {
        return str.replace(/([;&,\.\+\*\~':"\!\^#$%@\[\]\(\)=<>\|])/g, '\\$1');
    }

    function toggleVisibilityFilter(ctrlToEnable, ctrToDisable) {
        if (ctrlToEnable.hasClass("out")) {
            ctrlToEnable.removeClass("out").addClass("in");
            ctrToDisable.removeClass("in").addClass("out");
            filter();
        }
    }

    controls.visibility.publicOnly.click(function () {
        toggleVisibilityFilter(controls.visibility.publicOnly, controls.visibility.all);
    });

    controls.visibility.all.click(function () {
        toggleVisibilityFilter(controls.visibility.all, controls.visibility.publicOnly);
    });

    function exposeMember(jqElem) {
        var jqElemParent = jqElem.parent(),
            parentName = jqElemParent.attr("name"),
            ancestorName = /^([^#]*)(#.*)?$/gi.exec(parentName)[1];

        // switch visibility filter if necessary
        if (jqElemParent.attr("visbl") == "prt") {
            toggleVisibilityFilter(controls.visibility.all, controls.visibility.publicOnly);
        }

        // toggle appropriate ancestor filter buttons
        if (ancestorName) {
            $("#filterby li.out[name='" + ancestorName + "']").removeClass("out").addClass("in");
        }

        filter();
        jqElemParent.addClass("selected");
        commentToggleFct(jqElemParent);
        $("#content-scroll-container").animate({scrollTop: $("#content-scroll-container").scrollTop() + jqElemParent.offset().top - $("#search").height() - 23}, 1000);
    }

    var isHiddenClass = function (name) {
        return name == 'scala.Any' ||
            name == 'scala.AnyRef';
    };

    var isHidden = function (elem) {
        return $(elem).attr("data-hidden") == 'true';
    };

    $("#linearization li:gt(0)").filter(function () {
        return isHiddenClass($(this).attr("name"));
    }).removeClass("in").addClass("out");

    $("#implicits li").filter(function () {
        return isHidden(this);
    }).removeClass("in").addClass("out");

    $("#memberfilter > i.arrow").click(function () {
        $(this).toggleClass("rotate");
        $("#filterby").toggle();
    });

    // Pre-filter members
    filter();

    // Member filter box
    var input = $("#memberfilter input");
    input.bind("keyup", function (event) {

        switch (event.keyCode) {

            case 27: // escape key
                input.val("");
                filter(true);
                break;

            case 38: // up
                input.val("");
                filter(false);
                window.scrollTo(0, $("body").offset().top);
                input.focus();
                break;

            case 33: //page up
                input.val("");
                filter(false);
                break;

            case 34: //page down
                input.val("");
                filter(false);
                break;

            default:
                window.scrollTo(0, $("#mbrsel").offset().top - 130);
                filter(true);
                break;

        }
    });
    input.focus(function (event) {
        input.select();
    });
    $("#memberfilter > .clear").click(function () {
        $("#memberfilter input").attr("value", "");
        $(this).hide();
        filter();
    });
    $(document).keydown(function (event) {
        if (event.keyCode == 9) { // tab
            $("#index-input", window.parent.document).focus();
            input.attr("value", "");
            return false;
        }
    });

    $("#linearization li").click(function () {
        if ($(this).hasClass("in")) {
            $(this).removeClass("in");
            $(this).addClass("out");
        } else if ($(this).hasClass("out")) {
            $(this).removeClass("out");
            $(this).addClass("in");
        }
        filter();
    });

    $("#implicits li").click(function () {
        if ($(this).hasClass("in")) {
            $(this).removeClass("in");
            $(this).addClass("out");
        } else if ($(this).hasClass("out")) {
            $(this).removeClass("out");
            $(this).addClass("in");
        }
        filter();
    });

    $("#mbrsel > div > div.ancestors > ol > li.hideall").click(function () {
        $("#linearization li.in").removeClass("in").addClass("out");
        $("#linearization li:first").removeClass("out").addClass("in");
        $("#implicits li.in").removeClass("in").addClass("out");

        if ($(this).hasClass("out") && $("#mbrsel > div > div.ancestors > ol > li.showall").hasClass("in")) {
            $(this).removeClass("out").addClass("in");
            $("#mbrsel > div > div.ancestors > ol > li.showall").removeClass("in").addClass("out");
        }

        filter();
    })
    $("#mbrsel > div > div.ancestors > ol > li.showall").click(function () {
        var filteredLinearization =
            $("#linearization li.out").filter(function () {
                return !isHiddenClass($(this).attr("name"));
            });
        filteredLinearization.removeClass("out").addClass("in");

        var filteredImplicits =
            $("#implicits li.out").filter(function () {
                return !isHidden(this);
            });
        filteredImplicits.removeClass("out").addClass("in");

        if ($(this).hasClass("out") && $("#mbrsel > div > div.ancestors > ol > li.hideall").hasClass("in")) {
            $(this).removeClass("out").addClass("in");
            $("#mbrsel > div > div.ancestors > ol > li.hideall").removeClass("in").addClass("out");
        }

        filter();
    });
    $("#order > ol > li.alpha").click(function () {
        if ($(this).hasClass("out"))
            orderAlpha();
    })
    $("#order > ol > li.inherit").click(function () {
        if ($(this).hasClass("out"))
            orderInherit();
    });
    $("#order > ol > li.group").click(function () {
        if ($(this).hasClass("out"))
            orderGroup();
    });
    $("#groupedMembers").hide();

    initInherit();

    // Create tooltips
    $(".extype").add(".defval").tooltip({
        tip: "#tooltip",
        position: "top center",
        predelay: 500,
        onBeforeShow: function (ev) {
            $(this.getTip()).text(this.getTrigger().attr("name"));
        }
    });

    /* Add toggle arrows */
    $("#template li[fullComment=yes] .modifier_kind").addClass("closed");

    function commentToggleFct(element) {
        $("#template li.selected").removeClass("selected");
        element.toggleClass("open");
        var signature = element.find(".modifier_kind")
        var shortComment = element.find(".shortcomment");
        var fullComment = element.find(".fullcomment");
        var vis = $(":visible", fullComment);
        signature.toggleClass("closed").toggleClass("opened");
        if (vis.length > 0) {
            if (!isMobile()) {
                shortComment.slideDown(100);
                fullComment.slideUp(100);
            } else {
                fullComment.hide();
                shortComment.show();
            }
        } else {
            if (!isMobile()) {
                shortComment.slideUp(100);
                fullComment.slideDown(100);
            } else {
                shortComment.hide();
                fullComment.show();
            }
        }
    };

    $("#template li[fullComment=yes]").click(function () {
        var sel = window.getSelection().toString();
        if (!sel) commentToggleFct($(this));
    });

    /* Linear super types and known subclasses */
    function toggleShowContentFct(e) {
        e.toggleClass("open");
        var content = $(".hiddenContent", e);
        if (content.is(':visible')) {
            if (!isMobile()) content.slideUp(100);
            else content.hide();
        } else {
            if (!isMobile()) content.slideDown(100);
            else content.show();
        }
    };

    $(".toggleContainer:not(.diagram-container):not(.full-signature-block)").click(function () {
        toggleShowContentFct($(this));
    });

    $(".toggleContainer.full-signature-block").click(function () {
        toggleShowContentFct($(this));
        return false;
    });

    if ($("#order > ol > li.group").length == 1) {
        orderGroup();
    }
    ;

    function findElementByHash(locationHash) {
        var temp = locationHash.replace('#', '');
        var memberSelector = '#' + escapeJquery(temp);
        return $(memberSelector);
    }

    // highlight and jump to selected member if an anchor is provided
    if (window.location.hash) {
        var jqElem = findElementByHash(window.location.hash);
        if (jqElem.length > 0)
            exposeMember(jqElem);
    }

    $("#template span.permalink").click(function (e) {
        e.preventDefault();
        var href = $("a", this).attr("href");
        if (href.indexOf("#") != -1) {
            var hash = href.split("#").pop()
            try {
                window.history.pushState({}, "", "#" + hash)
            } catch (e) {
                // fallback for file:// URLs, has worse scrolling behavior
                location.hash = hash;
            }
            exposeMember(findElementByHash(hash))
        }
        return false;
    });

    $("#mbrsel-input").on("input", function () {
        if ($(this).val().length > 0)
            $("#memberfilter > .clear").show();
        else
            $("#memberfilter > .clear").hide();
    });
});

function orderAlpha() {
    $("#order > ol > li.alpha").removeClass("out").addClass("in");
    $("#order > ol > li.inherit").removeClass("in").addClass("out");
    $("#order > ol > li.group").removeClass("in").addClass("out");
    $("#template > div.parent").hide();
    $("#template > div.conversion").hide();
    $("#mbrsel > div.ancestors").show();
    filter();
};

function orderInherit() {
    $("#order > ol > li.inherit").removeClass("out").addClass("in");
    $("#order > ol > li.alpha").removeClass("in").addClass("out");
    $("#order > ol > li.group").removeClass("in").addClass("out");
    $("#template > div.parent").show();
    $("#template > div.conversion").show();
    $("#mbrsel > div.ancestors").hide();
    filter();
};

function orderGroup() {
    $("#order > ol > li.group").removeClass("out").addClass("in");
    $("#order > ol > li.alpha").removeClass("in").addClass("out");
    $("#order > ol > li.inherit").removeClass("in").addClass("out");
    $("#template > div.parent").hide();
    $("#template > div.conversion").hide();
    $("#mbrsel > div.ancestors").show();
    filter();
};

/** Prepares the DOM for inheritance-based display. To do so it will:
 *  - hide all statically-generated parents headings;
 *  - copy all members from the value and type members lists (flat members) to corresponding lists nested below the
 *    parent headings (inheritance-grouped members);
 *  - initialises a control variable used by the filter method to control whether filtering happens on flat members
 *    or on inheritance-grouped members. */
function initInherit() {
    // inheritParents is a map from fully-qualified names to the DOM node of parent headings.
    var inheritParents = new Object();
    var groupParents = new Object();
    $("#inheritedMembers > div.parent").each(function () {
        inheritParents[$(this).attr("name")] = $(this);
    });
    $("#inheritedMembers > div.conversion").each(function () {
        inheritParents[$(this).attr("name")] = $(this);
    });
    $("#groupedMembers > div.group").each(function () {
        groupParents[$(this).attr("name")] = $(this);
    });

    $("#types > ol > li").each(function () {
        var mbr = $(this);
        this.mbrText = mbr.find("> .fullcomment .cmt").text();
        var qualName = mbr.attr("name");
        var owner = qualName.slice(0, qualName.indexOf("#"));
        var name = qualName.slice(qualName.indexOf("#") + 1);
        var inheritParent = inheritParents[owner];
        if (inheritParent != undefined) {
            var types = $("> .types > ol", inheritParent);
            if (types.length == 0) {
                inheritParent.append("<div class='types members'><h3>Type Members</h3><ol></ol></div>");
                types = $("> .types > ol", inheritParent);
            }
            var clone = mbr.clone();
            clone[0].mbrText = this.mbrText;
            types.append(clone);
        }
        var group = mbr.attr("group")
        var groupParent = groupParents[group];
        if (groupParent != undefined) {
            var types = $("> .types > ol", groupParent);
            if (types.length == 0) {
                groupParent.append("<div class='types members'><ol></ol></div>");
                types = $("> .types > ol", groupParent);
            }
            var clone = mbr.clone();
            clone[0].mbrText = this.mbrText;
            types.append(clone);
        }
    });

    $(".values > ol > li").each(function () {
        var mbr = $(this);
        this.mbrText = mbr.find("> .fullcomment .cmt").text();
        var qualName = mbr.attr("name");
        var owner = qualName.slice(0, qualName.indexOf("#"));
        var name = qualName.slice(qualName.indexOf("#") + 1);
        var inheritParent = inheritParents[owner];
        if (inheritParent != undefined) {
            var values = $("> .values > ol", inheritParent);
            if (values.length == 0) {
                inheritParent.append("<div class='values members'><h3>Value Members</h3><ol></ol></div>");
                values = $("> .values > ol", inheritParent);
            }
            var clone = mbr.clone();
            clone[0].mbrText = this.mbrText;
            values.append(clone);
        }
        var group = mbr.attr("group")
        var groupParent = groupParents[group];
        if (groupParent != undefined) {
            var values = $("> .values > ol", groupParent);
            if (values.length == 0) {
                groupParent.append("<div class='values members'><ol></ol></div>");
                values = $("> .values > ol", groupParent);
            }
            var clone = mbr.clone();
            clone[0].mbrText = this.mbrText;
            values.append(clone);
        }
    });
    $("#inheritedMembers > div.parent").each(function () {
        if ($("> div.members", this).length == 0) {
            $(this).remove();
        }
        ;
    });
    $("#inheritedMembers > div.conversion").each(function () {
        if ($("> div.members", this).length == 0) {
            $(this).remove();
        }
        ;
    });
    $("#groupedMembers > div.group").each(function () {
        if ($("> div.members", this).length == 0) {
            $(this).remove();
        }
        ;
    });
};

/* filter used to take boolean scrollToMember */
function filter() {
    var query = $.trim($("#memberfilter input").val()).toLowerCase();
    query = query.replace(/[-[\]{}()*+?.,\\^$|#]/g, "\\$&").replace(/\s+/g, "|");
    var queryRegExp = new RegExp(query, "i");
    var privateMembersHidden = $("#visbl > ol > li.public").hasClass("in");
    var orderingAlphabetic = $("#order > ol > li.alpha").hasClass("in");
    var orderingInheritance = $("#order > ol > li.inherit").hasClass("in");
    var orderingGroups = $("#order > ol > li.group").hasClass("in");
    var hiddenSuperclassElementsLinearization = orderingInheritance ? $("#linearization > li:gt(0)") : $("#linearization > li.out");
    var hiddenSuperclassesLinearization = hiddenSuperclassElementsLinearization.map(function () {
        return $(this).attr("name");
    }).get();
    var hiddenSuperclassElementsImplicits = orderingInheritance ? $("#implicits > li") : $("#implicits > li.out");
    var hiddenSuperclassesImplicits = hiddenSuperclassElementsImplicits.map(function () {
        return $(this).attr("name");
    }).get();

    var hideInheritedMembers;

    if (orderingAlphabetic) {
        $("#allMembers").show();
        $("#inheritedMembers").hide();
        $("#groupedMembers").hide();
        hideInheritedMembers = true;
        $("#allMembers > .members").each(filterFunc);
    } else if (orderingGroups) {
        $("#groupedMembers").show();
        $("#inheritedMembers").hide();
        $("#allMembers").hide();
        hideInheritedMembers = true;
        $("#groupedMembers  > .group > .members").each(filterFunc);
        $("#groupedMembers  > div.group").each(function () {
            $(this).show();
            if ($("> div.members", this).not(":hidden").length == 0) {
                $(this).hide();
            } else {
                $(this).show();
            }
        });
    } else if (orderingInheritance) {
        $("#inheritedMembers").show();
        $("#groupedMembers").hide();
        $("#allMembers").hide();
        hideInheritedMembers = false;
        $("#inheritedMembers > .parent > .members").each(filterFunc);
        $("#inheritedMembers > .conversion > .members").each(filterFunc);
    }


    function filterFunc() {
        var membersVisible = false;
        var members = $(this);
        members.find("> ol > li").each(function () {
            var mbr = $(this);
            if (privateMembersHidden && mbr.attr("visbl") == "prt") {
                mbr.hide();
                return;
            }
            var name = mbr.attr("name");
            // Owner filtering must not happen in "inherited from" member lists
            if (hideInheritedMembers) {
                var ownerIndex = name.indexOf("#");
                if (ownerIndex < 0) {
                    ownerIndex = name.lastIndexOf(".");
                }
                var owner = name.slice(0, ownerIndex);
                for (var i = 0; i < hiddenSuperclassesLinearization.length; i++) {
                    if (hiddenSuperclassesLinearization[i] == owner) {
                        mbr.hide();
                        return;
                    }
                }
                ;
                for (var i = 0; i < hiddenSuperclassesImplicits.length; i++) {
                    if (hiddenSuperclassesImplicits[i] == owner) {
                        mbr.hide();
                        return;
                    }
                }
                ;
            }
            if (query && !(queryRegExp.test(name) || queryRegExp.test(this.mbrText))) {
                mbr.hide();
                return;
            }
            mbr.show();
            membersVisible = true;
        });

        if (membersVisible)
            members.show();
        else
            members.hide();
    };

    return false;
};

/** Check if user agent is associated with a known mobile browser */
function isMobile() {
    return /Android|webOS|Mobi|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent);
}


```

## Source file: scala/docs/api/lib/scheduler.js

{
  "path": "scala/docs/api/lib/scheduler.js",
  "blob_sha": "8fd16df98b1d46f6b44f8e691b0a5f50e38d2043",
  "sha256": "73df33cf61f9e3f65fb7ed42b17a91134331e748f40564c4687fbc96ee68049e",
  "lines": 113,
  "bytes": 3515,
  "selection_score": 0,
  "keyword_line_counts": {},
  "url": "https://github.com/grigala/3DMMDepthFitting/blob/bf48b9d1cdf495e471bdb8cb6f9c5db7271d1698/scala/docs/api/lib/scheduler.js",
  "function_signature_candidates": []
}

```text

// © 2010 EPFL/LAMP
// code by Gilles Dubochet, Felix Mulder

function Scheduler() {
    var scheduler = this;
    var resolution = 0;
    this.timeout = undefined;
    this.queues = new Array(0); // an array of work packages indexed by index in the labels table.
    this.labels = new Array(0); // an indexed array of labels indexed by priority. This should be short.

    this.label = function (name, priority) {
        this.name = name;
        this.priority = priority;
    }

    this.work = function (fn, self, args) {
        this.fn = fn;
        this.self = self;
        this.args = args;
    }

    this.addLabel = function (name, priority) {
        var idx = 0;
        while (idx < scheduler.queues.length && scheduler.labels[idx].priority <= priority) {
            idx = idx + 1;
        }
        scheduler.labels.splice(idx, 0, new scheduler.label(name, priority));
        scheduler.queues.splice(idx, 0, new Array(0));
    }

    this.clearLabel = function (name) {
        var idx = scheduler.indexOf(name);
        if (idx != -1) {
            scheduler.labels.splice(idx, 1);
            scheduler.queues.splice(idx, 1);
        }
    }

    this.nextWork = function () {
        var fn = undefined;
        var idx = 0;
        while (idx < scheduler.queues.length && scheduler.queues[idx].length == 0) {
            idx = idx + 1;
        }

        if (idx < scheduler.queues.length && scheduler.queues[idx].length > 0)
            var fn = scheduler.queues[idx].shift();

        return fn;
    }

    this.add = function (labelName, fn, self, args) {
        var doWork = function () {
            scheduler.timeout = setTimeout(function () {
                var work = scheduler.nextWork();
                if (work != undefined) {
                    if (work.args == undefined) {
                        work.args = new Array(0);
                    }
                    work.fn.apply(work.self, work.args);
                    doWork();
                } else {
                    scheduler.timeout = undefined;
                }
            }, resolution);
        }

        var idx = scheduler.indexOf(labelName)
        if (idx != -1) {
            scheduler.queues[idx].push(new scheduler.work(fn, self, args));
            if (scheduler.timeout == undefined) doWork();
        } else {
            throw("queue for add is non-existent");
        }
    }

    this.clear = function (labelName) {
        scheduler.queues[scheduler.indexOf(labelName)] = new Array();
    }

    this.indexOf = function (label) {
        var idx = 0;
        while (idx < scheduler.labels.length && scheduler.labels[idx].name != label)
            idx++;

        return idx < scheduler.queues.length && scheduler.labels[idx].name == label ? idx : -1;
    }

    this.queueEmpty = function (label) {
        var idx = scheduler.indexOf(label);
        if (idx != -1)
            return scheduler.queues[idx].length == 0;
        else
            throw("queue for label '" + label + "' is non-existent");
    }

    this.scheduleLast = function (label, fn) {
        if (scheduler.queueEmpty(label)) {
            fn();
        } else {
            scheduler.add(label, function () {
                scheduler.scheduleLast(label, fn);
            });
        }
    }

    this.numberOfJobs = function (label) {
        var index = scheduler.indexOf(label);
        if (index == -1) throw("queue for label '" + label + "' non-existent");

        return scheduler.queues[index].length;
    }
};


```
